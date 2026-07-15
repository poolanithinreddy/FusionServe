#include "fusionserve_client.hpp"

#include <curl/curl.h>

#include <atomic>
#include <cstring>
#include <sstream>

namespace fusionserve {
namespace detail {

// Escape a string for embedding in JSON.
static std::string json_escape(const std::string& s) {
    std::string out;
    out.reserve(s.size() + 8);
    for (char c : s) {
        switch (c) {
            case '"': out += "\\\""; break;
            case '\\': out += "\\\\"; break;
            case '\n': out += "\\n"; break;
            case '\r': out += "\\r"; break;
            case '\t': out += "\\t"; break;
            default: out += c;
        }
    }
    return out;
}

std::string build_chat_body(const ChatRequest& req, bool force_stream) {
    std::ostringstream os;
    os << "{\"model\":\"" << json_escape(req.model) << "\",";
    os << "\"stream\":" << ((req.stream || force_stream) ? "true" : "false") << ",";
    if (req.max_tokens) os << "\"max_tokens\":" << *req.max_tokens << ",";
    if (req.temperature) os << "\"temperature\":" << *req.temperature << ",";
    os << "\"messages\":[";
    for (size_t i = 0; i < req.messages.size(); ++i) {
        const auto& m = req.messages[i];
        os << "{\"role\":\"" << json_escape(m.role) << "\",\"content\":\""
           << json_escape(m.content) << "\"}";
        if (i + 1 < req.messages.size()) os << ",";
    }
    os << "]}";
    return os.str();
}

// Extract the string value for the first occurrence of "key":"...".
// Deliberately minimal — sufficient for the fields we read; for arbitrary JSON
// use a real parser.
std::string extract_string_field(const std::string& json, const std::string& key) {
    const std::string needle = "\"" + key + "\"";
    size_t pos = json.find(needle);
    if (pos == std::string::npos) return "";
    pos = json.find(':', pos + needle.size());
    if (pos == std::string::npos) return "";
    // skip whitespace
    ++pos;
    while (pos < json.size() && (json[pos] == ' ' || json[pos] == '\t')) ++pos;
    if (pos >= json.size() || json[pos] != '"') return "";
    ++pos;
    std::string out;
    while (pos < json.size()) {
        char c = json[pos];
        if (c == '\\' && pos + 1 < json.size()) {
            char n = json[pos + 1];
            switch (n) {
                case 'n': out += '\n'; break;
                case 't': out += '\t'; break;
                case 'r': out += '\r'; break;
                default: out += n;
            }
            pos += 2;
            continue;
        }
        if (c == '"') break;
        out += c;
        ++pos;
    }
    return out;
}

bool parse_sse_line(const std::string& line, ChatChunk& out) {
    const std::string prefix = "data:";
    if (line.rfind(prefix, 0) != 0) return false;
    std::string payload = line.substr(prefix.size());
    // trim leading spaces
    size_t s = payload.find_first_not_of(" \t");
    if (s == std::string::npos) return false;
    payload = payload.substr(s);
    if (payload == "[DONE]") {
        out.done = true;
        out.content.clear();
        return true;
    }
    std::string delta = extract_string_field(payload, "content");
    if (delta.empty()) return false;
    out.content = delta;
    out.done = false;
    return true;
}

}  // namespace detail

// ---- libcurl write callbacks ---------------------------------------------
namespace {
size_t write_to_string(char* ptr, size_t size, size_t nmemb, void* userdata) {
    auto* out = static_cast<std::string*>(userdata);
    out->append(ptr, size * nmemb);
    return size * nmemb;
}

struct StreamCtx {
    const ChatChunkCallback* cb;
    const bool* cancel;
    std::string buffer;
};

size_t write_stream(char* ptr, size_t size, size_t nmemb, void* userdata) {
    auto* ctx = static_cast<StreamCtx*>(userdata);
    if (ctx->cancel && *ctx->cancel) {
        return 0;  // returning < received bytes aborts the transfer
    }
    ctx->buffer.append(ptr, size * nmemb);
    size_t nl;
    while ((nl = ctx->buffer.find('\n')) != std::string::npos) {
        std::string line = ctx->buffer.substr(0, nl);
        ctx->buffer.erase(0, nl + 1);
        if (!line.empty() && line.back() == '\r') line.pop_back();
        ChatChunk chunk;
        if (detail::parse_sse_line(line, chunk)) {
            (*ctx->cb)(chunk);
            if (chunk.done) break;
        }
    }
    return size * nmemb;
}

size_t header_capture(char* buffer, size_t size, size_t nitems, void* userdata) {
    auto* rid = static_cast<std::string*>(userdata);
    std::string h(buffer, size * nitems);
    const std::string key = "x-request-id:";
    if (h.size() > key.size()) {
        std::string lower = h;
        for (auto& c : lower) c = static_cast<char>(::tolower(c));
        if (lower.rfind(key, 0) == 0) {
            std::string v = h.substr(key.size());
            size_t s = v.find_first_not_of(" \t");
            size_t e = v.find_last_not_of(" \r\n\t");
            if (s != std::string::npos) *rid = v.substr(s, e - s + 1);
        }
    }
    return size * nitems;
}
}  // namespace

// ---- client ---------------------------------------------------------------
FusionServeClient::FusionServeClient(const ClientConfig& config) : config_(config) {
    static std::atomic<bool> curl_inited{false};
    if (!curl_inited.exchange(true)) {
        curl_global_init(CURL_GLOBAL_DEFAULT);
    }
    easy_ = curl_easy_init();
    if (!easy_) throw std::runtime_error("failed to init curl");
}

FusionServeClient::~FusionServeClient() {
    if (easy_) curl_easy_cleanup(static_cast<CURL*>(easy_));
}

bool FusionServeClient::healthy() {
    std::lock_guard<std::mutex> lock(mu_);
    CURL* h = static_cast<CURL*>(easy_);
    curl_easy_reset(h);
    std::string body;
    curl_easy_setopt(h, CURLOPT_URL, (config_.base_url + "/healthz").c_str());
    curl_easy_setopt(h, CURLOPT_WRITEFUNCTION, write_to_string);
    curl_easy_setopt(h, CURLOPT_WRITEDATA, &body);
    curl_easy_setopt(h, CURLOPT_TIMEOUT_MS, config_.timeout_ms);
    CURLcode rc = curl_easy_perform(h);
    long status = 0;
    curl_easy_getinfo(h, CURLINFO_RESPONSE_CODE, &status);
    return rc == CURLE_OK && status == 200;
}

FusionServeClient::HttpResponse FusionServeClient::post(const std::string& path,
                                                        const std::string& body,
                                                        const std::string& request_id) {
    std::lock_guard<std::mutex> lock(mu_);
    CURL* h = static_cast<CURL*>(easy_);
    curl_easy_reset(h);

    HttpResponse resp{0, "", ""};
    struct curl_slist* headers = nullptr;
    headers = curl_slist_append(headers, "Content-Type: application/json");
    std::string rid_header = "x-request-id: " + request_id;
    headers = curl_slist_append(headers, rid_header.c_str());

    curl_easy_setopt(h, CURLOPT_URL, (config_.base_url + path).c_str());
    curl_easy_setopt(h, CURLOPT_POSTFIELDS, body.c_str());
    curl_easy_setopt(h, CURLOPT_POSTFIELDSIZE, static_cast<long>(body.size()));
    curl_easy_setopt(h, CURLOPT_HTTPHEADER, headers);
    curl_easy_setopt(h, CURLOPT_WRITEFUNCTION, write_to_string);
    curl_easy_setopt(h, CURLOPT_WRITEDATA, &resp.body);
    curl_easy_setopt(h, CURLOPT_HEADERFUNCTION, header_capture);
    curl_easy_setopt(h, CURLOPT_HEADERDATA, &resp.request_id);
    curl_easy_setopt(h, CURLOPT_TIMEOUT_MS, config_.timeout_ms);
    curl_easy_setopt(h, CURLOPT_CONNECTTIMEOUT_MS, config_.connect_timeout_ms);

    CURLcode rc = curl_easy_perform(h);
    curl_slist_free_all(headers);
    if (rc != CURLE_OK) {
        throw FusionServeError(0, "connect", curl_easy_strerror(rc), request_id);
    }
    long status = 0;
    curl_easy_getinfo(h, CURLINFO_RESPONSE_CODE, &status);
    resp.status = static_cast<int>(status);
    if (resp.status < 200 || resp.status >= 300) {
        throw_from_response(resp);
    }
    return resp;
}

void FusionServeClient::throw_from_response(const HttpResponse& resp) {
    std::string code = detail::extract_string_field(resp.body, "code");
    std::string message = detail::extract_string_field(resp.body, "message");
    std::string rid = detail::extract_string_field(resp.body, "request_id");
    if (rid.empty()) rid = resp.request_id;
    throw FusionServeError(resp.status, code.empty() ? "unknown" : code,
                           message.empty() ? "request failed" : message, rid);
}

ImageResult FusionServeClient::classify(const std::string& model,
                                        const std::vector<std::uint8_t>& image) {
    // For the reference client we forward raw bytes as a JSON number array under
    // "inputs". A production client would send a proper KServe tensor.
    std::ostringstream os;
    os << "{\"model\":\"" << detail::json_escape(model) << "\",\"inputs\":[";
    for (size_t i = 0; i < image.size(); ++i) {
        os << static_cast<int>(image[i]);
        if (i + 1 < image.size()) os << ",";
    }
    os << "]}";
    auto resp = post("/v1/infer", os.str(), /*request_id=*/"");
    return ImageResult{model, resp.body, resp.request_id.empty()
                                             ? std::nullopt
                                             : std::optional<std::string>(resp.request_id)};
}

EmbeddingResult FusionServeClient::embed(const std::string& model, const std::string& text) {
    std::string body = "{\"model\":\"" + detail::json_escape(model) + "\",\"input\":\"" +
                       detail::json_escape(text) + "\"}";
    auto resp = post("/v1/embeddings", body, "");
    return EmbeddingResult{model, resp.body,
                           resp.request_id.empty()
                               ? std::nullopt
                               : std::optional<std::string>(resp.request_id)};
}

ChatResult FusionServeClient::chat(const ChatRequest& request) {
    std::string body = detail::build_chat_body(request, /*force_stream=*/false);
    auto resp = post("/v1/chat/completions", body, "");
    ChatResult r;
    r.raw_json = resp.body;
    r.content = detail::extract_string_field(resp.body, "content");
    if (!resp.request_id.empty()) r.request_id = resp.request_id;
    return r;
}

void FusionServeClient::stream_chat(const ChatRequest& request,
                                    const ChatChunkCallback& on_chunk, const bool* cancel) {
    std::lock_guard<std::mutex> lock(mu_);
    CURL* h = static_cast<CURL*>(easy_);
    curl_easy_reset(h);

    std::string body = detail::build_chat_body(request, /*force_stream=*/true);
    struct curl_slist* headers = nullptr;
    headers = curl_slist_append(headers, "Content-Type: application/json");
    headers = curl_slist_append(headers, "Accept: text/event-stream");

    StreamCtx ctx{&on_chunk, cancel, ""};
    curl_easy_setopt(h, CURLOPT_URL, (config_.base_url + "/v1/chat/completions").c_str());
    curl_easy_setopt(h, CURLOPT_POSTFIELDS, body.c_str());
    curl_easy_setopt(h, CURLOPT_POSTFIELDSIZE, static_cast<long>(body.size()));
    curl_easy_setopt(h, CURLOPT_HTTPHEADER, headers);
    curl_easy_setopt(h, CURLOPT_WRITEFUNCTION, write_stream);
    curl_easy_setopt(h, CURLOPT_WRITEDATA, &ctx);
    curl_easy_setopt(h, CURLOPT_TIMEOUT_MS, config_.timeout_ms);

    CURLcode rc = curl_easy_perform(h);
    curl_slist_free_all(headers);
    // CURLE_WRITE_ERROR is expected when the caller cancels via `*cancel`.
    if (rc != CURLE_OK && !(cancel && *cancel && rc == CURLE_WRITE_ERROR)) {
        throw FusionServeError(0, "stream", curl_easy_strerror(rc), "");
    }
}

std::future<ChatResult> FusionServeClient::chat_async(const ChatRequest& request) {
    return std::async(std::launch::async, [this, request]() { return chat(request); });
}

std::future<ImageResult> FusionServeClient::classify_async(
    const std::string& model, const std::vector<std::uint8_t>& image) {
    return std::async(std::launch::async,
                      [this, model, image]() { return classify(model, image); });
}

}  // namespace fusionserve
