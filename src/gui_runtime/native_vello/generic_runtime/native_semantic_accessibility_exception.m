#import <AppKit/AppKit.h>
#import <Foundation/Foundation.h>
#import <objc/runtime.h>

#include <math.h>
#include <stddef.h>
#include <string.h>

signed char radiant_native_convert_view_rect_to_screen(
    id view,
    id window,
    const NSRect *source,
    NSRect *out
) {
    if (out == NULL) {
        return 0;
    }
    *out = NSMakeRect(NAN, NAN, NAN, NAN);
    if (view == nil || window == nil || source == NULL) {
        return 0;
    }

    @try {
        if (![view isKindOfClass:[NSView class]]
            || ![window isKindOfClass:[NSWindow class]]
            || ![view respondsToSelector:@selector(convertRect:toView:)]
            || ![window respondsToSelector:@selector(convertRectToScreen:)]) {
            return 0;
        }

        NSRect window_rect = [(NSView *)view convertRect:*source toView:nil];
        NSRect screen_rect = [(NSWindow *)window convertRectToScreen:window_rect];
        if (!isfinite(screen_rect.origin.x)
            || !isfinite(screen_rect.origin.y)
            || !isfinite(screen_rect.size.width)
            || !isfinite(screen_rect.size.height)) {
            return 0;
        }
        *out = screen_rect;
        return 1;
    } @catch (...) {
        return 0;
    }
}

// Test-only Foundation double used to keep the Objective-C exception on the
// Objective-C side of the conversion helper boundary without constructing
// an AppKit object in the test harness.
@interface RadiantNativeCoordinateConversionTestObject : NSObject
@end

@implementation RadiantNativeCoordinateConversionTestObject

- (BOOL)isKindOfClass:(Class)aClass {
    Class view_class = objc_getClass("NSView");
    Class window_class = objc_getClass("NSWindow");
    if (aClass == view_class || aClass == window_class) {
        return YES;
    }
    return [super isKindOfClass:aClass];
}

- (BOOL)respondsToSelector:(SEL)selector {
    if (selector == @selector(convertRect:toView:)
        || selector == @selector(convertRectToScreen:)) {
        return YES;
    }
    return [super respondsToSelector:selector];
}

- (NSRect)convertRect:(NSRect)rect toView:(id)view {
    (void)view;
    return rect;
}

- (NSRect)convertRectToScreen:(NSRect)rect {
    [NSException raise:@"RadiantNativeCoordinateConversionTestException"
                format:@"convertRectToScreen: test fixture exception"];
    return rect;
}

@end

signed char radiant_native_test_convert_view_rect_to_screen(
    const NSRect *source,
    NSRect *out
) {
    if (out == NULL) {
        return 0;
    }
    *out = NSMakeRect(NAN, NAN, NAN, NAN);
    if (source == NULL) {
        return 0;
    }

    @autoreleasepool {
        signed char converted = 0;
        @try {
            RadiantNativeCoordinateConversionTestObject *object =
                [[RadiantNativeCoordinateConversionTestObject alloc] init];
            converted = radiant_native_convert_view_rect_to_screen(object, object, source, out);
        } @catch (...) {
            converted = 0;
        }
        if (converted == 0) {
            *out = NSMakeRect(NAN, NAN, NAN, NAN);
        }
        return converted;
    }
}

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
