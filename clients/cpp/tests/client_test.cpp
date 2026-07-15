// Unit tests for the FusionServe C++ client's pure logic (no network).
//
// These cover request serialization, SSE parsing, and error-field extraction —
// the parts most likely to regress. End-to-end tests against a live gateway live
// in the Python integration suite.
#include <gtest/gtest.h>

#include "fusionserve_client.hpp"

using namespace fusionserve;

TEST(BuildChatBody, MinimalRequest) {
    ChatRequest req;
    req.model = "qwen_small";
    req.messages = {{"user", "hi"}};
    std::string body = detail::build_chat_body(req, false);
    EXPECT_NE(body.find("\"model\":\"qwen_small\""), std::string::npos);
    EXPECT_NE(body.find("\"stream\":false"), std::string::npos);
    EXPECT_NE(body.find("\"role\":\"user\""), std::string::npos);
    EXPECT_NE(body.find("\"content\":\"hi\""), std::string::npos);
    EXPECT_EQ(body.find("max_tokens"), std::string::npos);
}

TEST(BuildChatBody, ForceStreamOverridesFlag) {
    ChatRequest req;
    req.model = "m";
    req.stream = false;
    std::string body = detail::build_chat_body(req, true);
    EXPECT_NE(body.find("\"stream\":true"), std::string::npos);
}

TEST(BuildChatBody, IncludesSamplingParams) {
    ChatRequest req;
    req.model = "m";
    req.max_tokens = 128;
    req.temperature = 0.5;
    std::string body = detail::build_chat_body(req, false);
    EXPECT_NE(body.find("\"max_tokens\":128"), std::string::npos);
    EXPECT_NE(body.find("\"temperature\":0.5"), std::string::npos);
}

TEST(BuildChatBody, EscapesQuotes) {
    ChatRequest req;
    req.model = "m";
    req.messages = {{"user", "she said \"hi\""}};
    std::string body = detail::build_chat_body(req, false);
    EXPECT_NE(body.find("\\\"hi\\\""), std::string::npos);
}

TEST(ExtractField, FindsMessageContent) {
    std::string json = R"({"choices":[{"message":{"role":"assistant","content":"hello world"}}]})";
    EXPECT_EQ(detail::extract_string_field(json, "content"), "hello world");
}

TEST(ExtractField, MissingKeyReturnsEmpty) {
    EXPECT_EQ(detail::extract_string_field("{}", "content"), "");
}

TEST(ParseSse, ContentDelta) {
    ChatChunk c;
    std::string line = R"(data: {"choices":[{"delta":{"content":"Hello"}}]})";
    ASSERT_TRUE(detail::parse_sse_line(line, c));
    EXPECT_EQ(c.content, "Hello");
    EXPECT_FALSE(c.done);
}

TEST(ParseSse, DoneMarker) {
    ChatChunk c;
    ASSERT_TRUE(detail::parse_sse_line("data: [DONE]", c));
    EXPECT_TRUE(c.done);
}

TEST(ParseSse, NonDataLineIgnored) {
    ChatChunk c;
    EXPECT_FALSE(detail::parse_sse_line(": keep-alive", c));
}

TEST(FusionServeError, CarriesCodeAndStatus) {
    FusionServeError e(503, "circuit_open", "breaker open", "rid-1");
    EXPECT_EQ(e.status, 503);
    EXPECT_EQ(e.code, "circuit_open");
    EXPECT_EQ(e.request_id, "rid-1");
}

int main(int argc, char** argv) {
    ::testing::InitGoogleTest(&argc, argv);
    return RUN_ALL_TESTS();
}
