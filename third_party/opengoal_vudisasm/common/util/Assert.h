#pragma once
#include <cstdio>
#include <cstdlib>
#include <string>
#define ASSERT(x) do { if (!(x)) { std::fprintf(stderr, "assertion failed: %s (%s:%d)\n", #x, __FILE__, __LINE__); std::abort(); } } while (0)
#define ASSERT_MSG(x, msg) do { if (!(x)) { std::fprintf(stderr, "assertion failed: %s (%s:%d)\n", std::string(msg).c_str(), __FILE__, __LINE__); std::abort(); } } while (0)
