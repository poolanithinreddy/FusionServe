// FusionServe C++ client.
//
// A thread-safe HTTP client for the FusionServe gateway with synchronous and
// asynchronous APIs, request deadlines, cancellation, and streaming chat.
// Transport is HTTP/1.1 via libcurl (keep-alive connection reuse). A gRPC
// transport is planned; see docs/limitations.md.
//
// RAII: each FusionServeClient owns its curl resources and cleans them up in the
// destructor. The class is safe to share across threads — each call uses its own
// curl easy handle from an internal pool guarded by a mutex.
#pragma once

#include <future>
#include <memory>
#include <mutex>
#include <string>

#include "request_types.hpp"

namespace fusionserve {

// Minimal helpers to build request JSON and parse the pieces we need without a
// third-party JSON dependency (kept small on purpose; swap in nlohmann/json if
// you already depend on it). Declared here for unit testing.
namespace detail {
std::string build_chat_body(const ChatRequest& req, bool force_stream);
std::string extract_string_field(const std::string& json, const std::string& key);
// Parse one SSE `data:` line into a chunk; returns false if the line is not a
// content delta (e.g. `[DONE]` or a keep-alive).
bool parse_sse_line(const std::string& line, ChatChunk& out);
}  // namespace detail

class FusionServeClient {
public:
    explicit FusionServeClient(const ClientConfig& config);
    ~FusionServeClient();

    FusionServeClient(const FusionServeClient&) = delete;
    FusionServeClient& operator=(const FusionServeClient&) = delete;

    // Liveness probe.
    bool healthy();

    // Non-LLM inference (routed to Triton).
    ImageResult classify(const std::string& model, const std::vector<std::uint8_t>& image);
    EmbeddingResult embed(const std::string& model, const std::string& text);

    // LLM chat (routed to Dynamo).
    ChatResult chat(const ChatRequest& request);

    // Streaming chat: `on_chunk` is invoked for each delta on the calling
    // thread. Set *cancel to true from another thread to abort mid-stream.
    void stream_chat(const ChatRequest& request, const ChatChunkCallback& on_chunk,
                     const bool* cancel = nullptr);

    // Asynchronous variants.
    std::future<ChatResult> chat_async(const ChatRequest& request);
    std::future<ImageResult> classify_async(const std::string& model,
                                             const std::vector<std::uint8_t>& image);

private:
    struct HttpResponse {
        int status;
        std::string body;
        std::string request_id;
    };

    // Perform a POST; throws FusionServeError on non-2xx.
    HttpResponse post(const std::string& path, const std::string& body,
                      const std::string& request_id);
    void throw_from_response(const HttpResponse& resp);

    ClientConfig config_;
    std::mutex mu_;  // guards the shared easy handle when reuse_connection is set
    void* easy_;     // CURL* (opaque to avoid leaking curl headers here)
};

}  // namespace fusionserve
