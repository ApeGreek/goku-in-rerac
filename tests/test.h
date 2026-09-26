#pragma once
#include <functional>
#include <stdexcept>
#include <string>
#include <vector>

struct TestCase { const char* name; std::function<void()> fn; };
std::vector<TestCase>& registry();
struct TestRegistrar { TestRegistrar(const char* n, std::function<void()> f) { registry().push_back({n, std::move(f)}); } };

#define TEST(name) static void name(); static TestRegistrar name##_reg(#name, name); static void name()
#define CHECK(cond) do { if (!(cond)) throw std::runtime_error(std::string(__FILE__) + ":" + std::to_string(__LINE__) + " CHECK(" #cond ")"); } while (0)
#define CHECK_THROWS(expr) do { bool threw = false; try { (void)(expr); } catch (...) { threw = true; } CHECK(threw); } while (0)
