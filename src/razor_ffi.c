#include <razor_api.h>
#include <razor_ffi.h>


extern void razor_log_to_rust(int level, const char* file, int line, const char* content);

static int log_callback(int level, const char* file, int line, const char* fmt, va_list vl) {

    char content[1024] = {0};
    vsnprintf(content, 1024, fmt, vl);
    razor_log_to_rust(level, file, line, content);
    return 0;
}

void razor_setup_log_ffi() {
    razor_setup_log(log_callback);
}

