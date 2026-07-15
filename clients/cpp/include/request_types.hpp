// FusionServe C++ client — request/response value types.
#pragma once

#include <cstdint>
#include <functional>
#include <optional>
#include <stdexcept>
#include <string>
#include <vector>

namespace fusionserve {

// Connection/behavior configuration for the client.
struct ClientConfig {
    std::string base_url = "http://localhost:8080";
    long timeout_ms = 35000;
    long connect_timeout_ms = 2000;
    bool reuse_connection = true;  // keep-alive via a persistent easy handle
};

// Mirrors the gateway's stable error contract so callers branch on `code`.
class FusionServeError : public std::runtime_error {
public:
    FusionServeError(int status, std::string code, const std::string& message,
                     std::string request_id)
        : std::runtime_error("[" + std::to_string(status) + " " + code + "] " + message),
          status(status),
          code(std::move(code)),
          request_id(std::move(request_id)) {}

    int status;
    std::string code;
    std::string request_id;
};

struct ImageResult {
    std::string model;
    std::string raw_json;              // full response body
    std::optional<std::string> request_id;
};

struct EmbeddingResult {
    std::string model;
    std::string raw_json;
    std::optional<std::string> request_id;
};

struct ChatMessage {
    std::string role;
    std::string content;
};

struct ChatRequest {
    std::string model;
    std::vector<ChatMessage> messages;
    std::optional<int> max_tokens;
    std::optional<double> temperature;
    bool stream = false;
};

struct ChatResult {
    std::string content;
    std::string raw_json;
    std::optional<std::string> request_id;
};

// A single streamed delta.
struct ChatChunk {
    std::string content;
    bool done = false;
};

using ChatChunkCallback = std::function<void(const ChatChunk&)>;

}  // namespace fusionserve
