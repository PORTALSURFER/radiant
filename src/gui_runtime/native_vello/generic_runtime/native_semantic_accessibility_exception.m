#import <Foundation/Foundation.h>

#include <stddef.h>
#include <string.h>

signed char radiant_native_bounded_ns_string_to_utf8(
    id value,
    unsigned char *out,
    size_t cap,
    size_t *len
) {
    if (len == NULL) {
        return 0;
    }
    *len = 0;
    if (value == nil || out == NULL) {
        return 0;
    }

    @try {
        if (![value isKindOfClass:[NSString class]]) {
            return 0;
        }
        NSString *string = (NSString *)value;
        if ([string length] > 1024) {
            return 0;
        }
        NSUInteger utf8_length = [string lengthOfBytesUsingEncoding:NSUTF8StringEncoding];
        if (utf8_length > 4096 || utf8_length > cap) {
            return 0;
        }
        NSData *data = [string dataUsingEncoding:NSUTF8StringEncoding];
        if (data == nil) {
            return 0;
        }
        NSUInteger data_length = [data length];
        if (data_length != utf8_length || data_length > 4096 || data_length > cap) {
            return 0;
        }
        if (data_length != 0) {
            const void *source = [data bytes];
            if (source == NULL) return 0;
            memcpy(out, source, data_length);
        }
        *len = (size_t)data_length;
        return 1;
    } @catch (...) {
        *len = 0;
        return 0;
    }
}

signed char radiant_native_attribute_is(
    id attribute,
    const unsigned char *expected,
    size_t expected_len
) {
    if (attribute == nil || expected == NULL) {
        return 0;
    }

    @try {
        if (![attribute isKindOfClass:[NSString class]]) {
            return 0;
        }
        NSString *expected_string = [[NSString alloc]
            initWithBytes:expected
            length:expected_len
            encoding:NSUTF8StringEncoding];
        if (expected_string == nil) {
            return 0;
        }
        return [(NSString *)attribute isEqualToString:expected_string] ? 1 : 0;
    } @catch (...) {
        return 0;
    }
}
