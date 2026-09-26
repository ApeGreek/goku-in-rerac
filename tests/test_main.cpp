// Tiny self-contained test runner; no third-party dependency needed yet.
#include "test.h"
#include <cstdio>

std::vector<TestCase>& registry() { static std::vector<TestCase> r; return r; }

int main() {
    int failed = 0;
    for (auto& t : registry()) {
        try { t.fn(); std::printf("ok   %s\n", t.name); }
        catch (const std::exception& e) { failed++; std::printf("FAIL %s: %s\n", t.name, e.what()); }
    }
    std::printf("%zu tests, %d failed\n", registry().size(), failed);
    return failed ? 1 : 0;
}
