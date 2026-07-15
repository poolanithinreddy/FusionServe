// Demo driver for the FusionServe C++ client.
//
//   fusionserve_demo [base_url]
//
// Exercises health, classification, unary chat, and streaming chat against a
// running gateway (mock or GPU stack).
#include <iostream>
#include <vector>

#include "fusionserve_client.hpp"

int main(int argc, char** argv) {
    fusionserve::ClientConfig cfg;
    if (argc > 1) cfg.base_url = argv[1];
    fusionserve::FusionServeClient client(cfg);

    if (!client.healthy()) {
        std::cerr << "gateway not healthy at " << cfg.base_url << "\n";
        return 1;
    }
    std::cout << "gateway healthy\n";

    try {
        std::vector<std::uint8_t> fake_image = {10, 20, 30, 40, 50};
        auto img = client.classify("resnet50", fake_image);
        std::cout << "classify response: " << img.raw_json.substr(0, 120) << "...\n";

        fusionserve::ChatRequest req;
        req.model = "qwen_small";
        req.messages = {{"user", "Say hello in five words."}};
        auto chat = client.chat(req);
        std::cout << "chat: " << chat.content << "\n";

        std::cout << "stream: ";
        client.stream_chat(req, [](const fusionserve::ChatChunk& c) {
            if (!c.done) std::cout << c.content << std::flush;
        });
        std::cout << "\n";
    } catch (const fusionserve::FusionServeError& e) {
        std::cerr << "error: " << e.what() << " (code=" << e.code << ")\n";
        return 2;
    }
    return 0;
}
