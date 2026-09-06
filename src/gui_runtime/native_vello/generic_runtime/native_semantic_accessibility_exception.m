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

static BOOL radiant_native_accessibility_string_matches(id actual, id expected) {
    if (actual == expected) {
        return YES;
    }
    if (actual == nil || expected == nil
        || ![actual isKindOfClass:[NSString class]]
        || ![expected isKindOfClass:[NSString class]]) {
        return NO;
    }
    return [(NSString *)actual isEqualToString:(NSString *)expected];
}

static BOOL radiant_native_accessibility_rect_is_finite(NSRect rect) {
    return isfinite(rect.origin.x)
        && isfinite(rect.origin.y)
        && isfinite(rect.size.width)
        && isfinite(rect.size.height)
        && rect.size.width >= 0.0
        && rect.size.height >= 0.0;
}

static BOOL radiant_native_accessibility_children_match(
    id actual,
    NSArray *expected
) {
    if (actual == nil || ![actual isKindOfClass:[NSArray class]]) {
        return NO;
    }
    NSArray *actual_children = (NSArray *)actual;
    if ([actual_children count] != [expected count]) {
        return NO;
    }
    for (NSUInteger index = 0; index < [expected count]; index += 1) {
        if ([actual_children objectAtIndex:index] != [expected objectAtIndex:index]) {
            return NO;
        }
    }
    return YES;
}

// Configure one custom element entirely through modern NSAccessibility
// properties.  This is intentionally the only Objective-C boundary for the
// pre-attachment property transaction; no legacy accessibility callback is
// consulted and accessibilityValue is left callback-backed.
signed char radiant_native_configure_accessibility_element(
    id element,
    id role,
    id parent,
    id children,
    const NSRect *frame,
    id label,
    id title,
    id help,
    signed char has_enabled,
    signed char enabled
) {
    if (element == nil || role == nil || parent == nil || children == nil || frame == NULL) {
        return 0;
    }

    @try {
        if (![element isKindOfClass:[NSAccessibilityElement class]]
            || ![role isKindOfClass:[NSString class]]
            || ![children isKindOfClass:[NSArray class]]
            || (label != nil && ![label isKindOfClass:[NSString class]])
            || (title != nil && ![title isKindOfClass:[NSString class]])
            || (help != nil && ![help isKindOfClass:[NSString class]])
            || !radiant_native_accessibility_rect_is_finite(*frame)
            || ![element respondsToSelector:@selector(isAccessibilityElement)]
            || ![element respondsToSelector:@selector(accessibilityRole)]
            || ![element respondsToSelector:@selector(setAccessibilityRole:)]
            || ![element respondsToSelector:@selector(accessibilityParent)]
            || ![element respondsToSelector:@selector(setAccessibilityParent:)]
            || ![element respondsToSelector:@selector(accessibilityChildren)]
            || ![element respondsToSelector:@selector(setAccessibilityChildren:)]
            || ![element respondsToSelector:@selector(accessibilityFrame)]
            || ![element respondsToSelector:@selector(setAccessibilityFrame:)]
            || ![element respondsToSelector:@selector(accessibilityLabel)]
            || ![element respondsToSelector:@selector(setAccessibilityLabel:)]
            || ![element respondsToSelector:@selector(accessibilityTitle)]
            || ![element respondsToSelector:@selector(setAccessibilityTitle:)]
            || ![element respondsToSelector:@selector(accessibilityHelp)]
            || ![element respondsToSelector:@selector(setAccessibilityHelp:)]) {
            return 0;
        }
        if (has_enabled != 0
            && (![element respondsToSelector:@selector(isAccessibilityEnabled)]
                || ![element respondsToSelector:@selector(setAccessibilityEnabled:)])) {
            return 0;
        }
        if (![element isAccessibilityElement]) {
            return 0;
        }

        [element setAccessibilityRole:role];
        [element setAccessibilityParent:parent];
        [element setAccessibilityChildren:(NSArray *)children];
        [element setAccessibilityFrame:*frame];
        [element setAccessibilityLabel:(NSString *)label];
        [element setAccessibilityTitle:(NSString *)title];
        [element setAccessibilityHelp:(NSString *)help];
        if (has_enabled != 0) {
            [element setAccessibilityEnabled:enabled != 0];
        }

        NSRect readback_frame = [element accessibilityFrame];
        if (![element isAccessibilityElement]
            || !radiant_native_accessibility_string_matches(
                [element accessibilityRole],
                role
            )
            || [element accessibilityParent] != parent
            || !radiant_native_accessibility_children_match(
                [element accessibilityChildren],
                (NSArray *)children
            )
            || !radiant_native_accessibility_rect_is_finite(readback_frame)
            || readback_frame.origin.x != frame->origin.x
            || readback_frame.origin.y != frame->origin.y
            || readback_frame.size.width != frame->size.width
            || readback_frame.size.height != frame->size.height
            || !radiant_native_accessibility_string_matches(
                [element accessibilityLabel],
                label
            )
            || !radiant_native_accessibility_string_matches(
                [element accessibilityTitle],
                title
            )
            || !radiant_native_accessibility_string_matches(
                [element accessibilityHelp],
                help
            )) {
            return 0;
        }
        if (has_enabled != 0 && [element isAccessibilityEnabled] != (enabled != 0)) {
            return 0;
        }
        return 1;
    } @catch (...) {
        return 0;
    }
}

static BOOL radiant_native_accessibility_children_host_is_supported(id host) {
    return host != nil
        && [host isKindOfClass:[NSView class]]
        && [host respondsToSelector:@selector(setAccessibilityChildren:)]
        && [host respondsToSelector:@selector(accessibilityChildren)];
}

static void radiant_native_attempt_clear_accessibility_children(id host) {
    @try {
        if (host != nil
            && [host isKindOfClass:[NSView class]]
            && [host respondsToSelector:@selector(setAccessibilityChildren:)]) {
            [host setAccessibilityChildren:nil];
        }
    } @catch (...) {
        // The host remains inert when even the cleanup setter throws.
    }
}

signed char radiant_native_set_accessibility_children(id host, id root) {
    if (host == nil) {
        return 0;
    }

    @try {
        if (root == nil || !radiant_native_accessibility_children_host_is_supported(host)) {
            radiant_native_attempt_clear_accessibility_children(host);
            return 0;
        }

        NSArray *children = @[root];
        [host setAccessibilityChildren:children];

        id readback = [host accessibilityChildren];
        if (![readback isKindOfClass:[NSArray class]]) {
            radiant_native_attempt_clear_accessibility_children(host);
            return 0;
        }
        NSArray *readback_children = (NSArray *)readback;
        if ([readback_children count] != 1 || [readback_children objectAtIndex:0] != root) {
            radiant_native_attempt_clear_accessibility_children(host);
            return 0;
        }
        return 1;
    } @catch (...) {
        radiant_native_attempt_clear_accessibility_children(host);
        return 0;
    }
}

signed char radiant_native_clear_accessibility_children(id host) {
    if (host == nil) {
        return 0;
    }

    @try {
        if (![host isKindOfClass:[NSView class]]
            || ![host respondsToSelector:@selector(setAccessibilityChildren:)]
            || ![host respondsToSelector:@selector(accessibilityChildren)]) {
            return 0;
        }
        [host setAccessibilityChildren:nil];
        id readback = [host accessibilityChildren];
        if (readback == nil) {
            return 1;
        }
        return [readback isKindOfClass:[NSArray class]]
            && [(NSArray *)readback count] == 0;
    } @catch (...) {
        return 0;
    }
}

// Test-only host fixtures.  They are kept on the Objective-C side so setter
// and getter exceptions never cross into the Rust test harness.
