#import <AppKit/AppKit.h>
#import <Foundation/Foundation.h>
#import <objc/runtime.h>
#include <math.h>

extern signed char radiant_native_convert_view_rect_to_screen(id, id, const NSRect *, NSRect *);

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
        RadiantNativeCoordinateConversionTestObject *object =
            [[RadiantNativeCoordinateConversionTestObject alloc] init];
        converted = radiant_native_convert_view_rect_to_screen(object, object, source, out);
        if (converted == 0) {
            *out = NSMakeRect(NAN, NAN, NAN, NAN);
        }
        return converted;
    }
}

@interface RadiantNativeAccessibilityChildrenUnsupportedView : NSView
@end

@implementation RadiantNativeAccessibilityChildrenUnsupportedView

- (BOOL)respondsToSelector:(SEL)selector {
    if (selector == @selector(setAccessibilityChildren:)
        || selector == @selector(accessibilityChildren)) {
        return NO;
    }
    return [super respondsToSelector:selector];
}

@end

@interface RadiantNativeAccessibilityChildrenThrowingSetterView : NSView
@end

@implementation RadiantNativeAccessibilityChildrenThrowingSetterView

- (void)setAccessibilityChildren:(NSArray *)children {
    (void)children;
    [NSException raise:@"RadiantNativeAccessibilityChildrenSetterException"
                format:@"setAccessibilityChildren: test fixture exception"];
}

@end

@interface RadiantNativeAccessibilityChildrenThrowingGetterView : NSView
@end

@implementation RadiantNativeAccessibilityChildrenThrowingGetterView

- (NSArray *)accessibilityChildren {
    [NSException raise:@"RadiantNativeAccessibilityChildrenGetterException"
                format:@"accessibilityChildren test fixture exception"];
    return nil;
}

@end

@interface RadiantNativeAccessibilityChildrenMismatchedView : NSView {
    BOOL radiant_native_children_cleared;
}
@end

@implementation RadiantNativeAccessibilityChildrenMismatchedView

- (void)setAccessibilityChildren:(NSArray *)children {
    radiant_native_children_cleared = children == nil || [children count] == 0;
}

- (NSArray *)accessibilityChildren {
    return radiant_native_children_cleared ? @[] : @[self];
}

@end

@interface RadiantNativeAccessibilityChildrenIgnoringClearView : NSView
@end

@implementation RadiantNativeAccessibilityChildrenIgnoringClearView

- (void)setAccessibilityChildren:(NSArray *)children {
    if (children != nil) {
        [super setAccessibilityChildren:children];
    }
}

@end

@interface RadiantNativeAccessibilityChildrenThrowingClearView : NSView
@end

@implementation RadiantNativeAccessibilityChildrenThrowingClearView

- (void)setAccessibilityChildren:(NSArray *)children {
    if (children == nil) {
        [NSException raise:@"RadiantNativeAccessibilityChildrenClearException"
                    format:@"setAccessibilityChildren:nil test fixture exception"];
    }
    [super setAccessibilityChildren:children];
}

@end

// Test-only element fixtures for the modern property transaction.  Each
// failure stays inside radiant_native_configure_accessibility_element's
// Objective-C exception/readback boundary.
@interface RadiantNativeAccessibilityElementThrowingSetter : NSAccessibilityElement
@end

@implementation RadiantNativeAccessibilityElementThrowingSetter

- (void)setAccessibilityRole:(NSString *)role {
    (void)role;
    [NSException raise:@"RadiantNativeAccessibilityElementSetterException"
                format:@"setAccessibilityRole: test fixture exception"];
}

@end

@interface RadiantNativeAccessibilityElementThrowingGetter : NSAccessibilityElement
@end

@implementation RadiantNativeAccessibilityElementThrowingGetter

- (NSString *)accessibilityRole {
    [NSException raise:@"RadiantNativeAccessibilityElementGetterException"
                format:@"accessibilityRole test fixture exception"];
    return nil;
}

@end

@interface RadiantNativeAccessibilityElementMismatchedParent : NSAccessibilityElement
@end

@implementation RadiantNativeAccessibilityElementMismatchedParent

- (id)accessibilityParent {
    return self;
}

@end

@interface RadiantNativeAccessibilityElementMismatchedChildren : NSAccessibilityElement
@end

@implementation RadiantNativeAccessibilityElementMismatchedChildren

- (NSArray *)accessibilityChildren {
    return @[self];
}

@end

@interface RadiantNativeAccessibilityElementFiltered : NSAccessibilityElement
@end

@implementation RadiantNativeAccessibilityElementFiltered

- (BOOL)isAccessibilityElement {
    return NO;
}

@end

@interface RadiantNativeAccessibilityElementUnsupported : NSAccessibilityElement
@end

@implementation RadiantNativeAccessibilityElementUnsupported

- (BOOL)respondsToSelector:(SEL)selector {
    if (selector == @selector(accessibilityRole)
        || selector == @selector(setAccessibilityRole:)) {
        return NO;
    }
    return [super respondsToSelector:selector];
}

@end

id __attribute__((ns_returns_retained)) radiant_native_test_make_accessibility_element(
    unsigned char kind
) {
    Class element_class = [NSAccessibilityElement class];
    switch (kind) {
        case 1:
            element_class = [RadiantNativeAccessibilityElementThrowingSetter class];
            break;
        case 2:
            element_class = [RadiantNativeAccessibilityElementThrowingGetter class];
            break;
        case 3:
            element_class = [RadiantNativeAccessibilityElementMismatchedParent class];
            break;
        case 4:
            element_class = [RadiantNativeAccessibilityElementMismatchedChildren class];
            break;
        case 5:
            element_class = [RadiantNativeAccessibilityElementFiltered class];
            break;
        case 6:
            element_class = [RadiantNativeAccessibilityElementUnsupported class];
            break;
        default:
            break;
    }
    return [[element_class alloc] init];
}

id __attribute__((ns_returns_retained)) radiant_native_test_make_accessibility_children_host(
    unsigned char kind
) {
    Class host_class = [NSView class];
    switch (kind) {
        case 1:
            host_class = [RadiantNativeAccessibilityChildrenUnsupportedView class];
            break;
        case 2:
            host_class = [RadiantNativeAccessibilityChildrenThrowingSetterView class];
            break;
        case 3:
            host_class = [RadiantNativeAccessibilityChildrenThrowingGetterView class];
            break;
        case 4:
            host_class = [RadiantNativeAccessibilityChildrenMismatchedView class];
            break;
        case 5:
            host_class = [RadiantNativeAccessibilityChildrenIgnoringClearView class];
            break;
        case 6:
            host_class = [RadiantNativeAccessibilityChildrenThrowingClearView class];
            break;
        default:
            break;
    }
    return [[host_class alloc] initWithFrame:NSMakeRect(0.0, 0.0, 320.0, 120.0)];
}
