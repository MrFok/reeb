#import <AppKit/AppKit.h>
#import <CoreGraphics/CoreGraphics.h>
#import <objc/runtime.h>
#include <stdlib.h>
#include <stdio.h>

// This tap only observes key codes and flags. It never reads text or changes events.
typedef void (*KeyCallback)(void *, uint16_t, uint32_t, uint64_t);
typedef struct {
    CFMachPortRef tap;
    CFRunLoopSourceRef source;
    CFRunLoopRef loop;
    KeyCallback callback;
    void *context;
} ReebInput;

static CGEventRef key_event(CGEventTapProxy proxy, CGEventType type, CGEventRef event, void *info) {
    (void)proxy;
    ReebInput *input = info;
    if (type == kCGEventTapDisabledByTimeout || type == kCGEventTapDisabledByUserInput) {
        input->callback(input->context, 0, type, 0);
        if (CGPreflightListenEventAccess()) {
            CGEventTapEnable(input->tap, true);
        } else {
            CFRunLoopStop(input->loop);
        }
    } else if (type == (CGEventType)NSEventTypeSystemDefined) {
        @autoreleasepool {
            NSEvent *auxiliary = [NSEvent eventWithCGEvent:event];
            // NX_SUBTYPE_AUX_CONTROL_BUTTONS carries brightness/audio key state
            // in data1, rather than a normal keyboard virtual key code.
            if (auxiliary && auxiliary.subtype == 8) {
                input->callback(input->context, 0, type, (uint32_t)auxiliary.data1);
            }
        }
    } else {
        input->callback(input->context,
                       (uint16_t)CGEventGetIntegerValueField(event, kCGKeyboardEventKeycode),
                       type, CGEventGetFlags(event));
    }
    return event;
}

bool reeb_input_allowed(void) { return CGPreflightListenEventAccess(); }

void reeb_input_request(void) {
    if (!CGRequestListenEventAccess()) {
        [NSWorkspace.sharedWorkspace openURL:[NSURL URLWithString:
            @"x-apple.systempreferences:com.apple.preference.security?Privacy_ListenEvent"]];
    }
}

void *reeb_input_create(KeyCallback callback, void *context) {
    if (!CGPreflightListenEventAccess()) return NULL;
    ReebInput *input = calloc(1, sizeof(ReebInput));
    if (!input) return NULL;
    input->callback = callback;
    input->context = context;
    input->loop = (CFRunLoopRef)CFRetain(CFRunLoopGetCurrent());
    CGEventMask mask = CGEventMaskBit(kCGEventKeyDown) | CGEventMaskBit(kCGEventKeyUp)
        | CGEventMaskBit(kCGEventFlagsChanged) | CGEventMaskBit(NSEventTypeSystemDefined);
    input->tap = CGEventTapCreate(kCGSessionEventTap, kCGHeadInsertEventTap,
                                 kCGEventTapOptionListenOnly, mask, key_event, input);
    if (!input->tap) {
        CFRelease(input->loop);
        free(input);
        return NULL;
    }
    input->source = CFMachPortCreateRunLoopSource(kCFAllocatorDefault, input->tap, 0);
    if (!input->source) {
        CFRelease(input->tap);
        CFRelease(input->loop);
        free(input);
        return NULL;
    }
    CFRunLoopAddSource(input->loop, input->source, kCFRunLoopCommonModes);
    return input;
}

void reeb_input_run(void *ptr) {
    ReebInput *input = ptr;
    CGEventTapEnable(input->tap, true);
    CFRunLoopRun();
    CGEventTapEnable(input->tap, false);
}

void reeb_input_stop(void *ptr) { CFRunLoopStop(((ReebInput *)ptr)->loop); }

void reeb_input_destroy(void *ptr) {
    ReebInput *input = ptr;
    CFRunLoopRemoveSource(input->loop, input->source, kCFRunLoopCommonModes);
    CFMachPortInvalidate(input->tap);
    CFRelease(input->source);
    CFRelease(input->tap);
    CFRelease(input->loop);
    free(input);
}

typedef void (*MenuCallback)(void *, uint32_t);
@interface ReebMenu : NSObject
@property(nonatomic) MenuCallback callback;
@property(nonatomic) void *context;
@property(nonatomic, strong) NSStatusItem *status;
@property(nonatomic, strong) NSMenuItem *toggle;
@property(nonatomic, strong) NSMenu *previousMenu;
@property(nonatomic, strong) id launchObserver;
@property(nonatomic) BOOL enabled;
- (void)action:(NSMenuItem *)sender;
- (void)installStatusItem;
@end

static __weak ReebMenu *current_menu;
static BOOL (*previous_reopen)(id, SEL, NSApplication *, BOOL);
void reeb_window_show(void);
static NSMenuItem *item(ReebMenu *owner, NSString *title, NSInteger tag, NSString *key);

// winit owns the application delegate and has no Dock-reopen handler. Extend
// only that delegate class, preserving any existing implementation on teardown.
static BOOL reopen(id delegate, SEL selector, NSApplication *app, BOOL visible) {
    ReebMenu *owner = current_menu;
    if (owner) {
        reeb_window_show();
        owner.callback(owner.context, 0);
        return NO;
    }
    return previous_reopen ? previous_reopen(delegate, selector, app, visible) : YES;
}

@implementation ReebMenu
- (void)installStatusItem {
    if (self.status || !NSApp.isRunning) return;
    self.status = [NSStatusBar.systemStatusBar statusItemWithLength:NSVariableStatusItemLength];
    self.status.autosaveName = @"reeb-status";
    self.status.visible = YES;
    // Vector keycap face, matching assets/ui/reeb-menubar.svg. AppKit redraws
    // these paths at the menu bar's scale rather than enlarging a bitmap.
    NSImage *image = [NSImage imageWithSize:NSMakeSize(18, 18) flipped:NO
        drawingHandler:^BOOL(NSRect bounds) {
            (void)bounds;
            [NSColor.whiteColor setStroke];
            NSBezierPath *outline = [NSBezierPath bezierPathWithRoundedRect:
                NSMakeRect(1.75, 1.75, 14.5, 14.5) xRadius:3.75 yRadius:3.75];
            outline.lineWidth = 1.5;
            [outline stroke];
            NSBezierPath *eyes = [NSBezierPath bezierPath];
            eyes.lineWidth = 1.5;
            eyes.lineCapStyle = NSLineCapStyleRound;
            [eyes moveToPoint:NSMakePoint(5.2, 9.9)];
            [eyes curveToPoint:NSMakePoint(7.7, 9.9)
                controlPoint1:NSMakePoint(5.9, 11.4) controlPoint2:NSMakePoint(7.0, 11.4)];
            [eyes moveToPoint:NSMakePoint(10.3, 9.9)];
            [eyes curveToPoint:NSMakePoint(12.8, 9.9)
                controlPoint1:NSMakePoint(11.0, 11.4) controlPoint2:NSMakePoint(12.1, 11.4)];
            [eyes stroke];
            return YES;
        }];
    image.template = YES;
    self.status.button.image = image;
    self.status.button.imagePosition = NSImageOnly;
    self.status.button.accessibilityLabel = @"reeb keyboard sounds";
    NSMenu *menu = [[NSMenu alloc] initWithTitle:@"reeb"];
    [menu addItem:item(self, @"Open reeb", 0, @"")];
    [menu addItem:item(self, @"Settings…", 1, @"")];
    [menu addItem:NSMenuItem.separatorItem];
    self.toggle = item(self, self.enabled ? @"Pause Keyboard Sounds" : @"Resume Keyboard Sounds", 2, @"");
    [menu addItem:self.toggle];
    [menu addItem:NSMenuItem.separatorItem];
    [menu addItem:item(self, @"Quit reeb", 3, @"")];
    self.status.menu = menu;
}
- (void)action:(NSMenuItem *)sender {
    if (sender.tag == 0 || sender.tag == 1) reeb_window_show();
    self.callback(self.context, (uint32_t)sender.tag);
}
@end

static NSMenuItem *item(ReebMenu *owner, NSString *title, NSInteger tag, NSString *key) {
    NSMenuItem *entry = [[NSMenuItem alloc] initWithTitle:title action:@selector(action:) keyEquivalent:key];
    entry.target = owner;
    entry.tag = tag;
    return entry;
}

void *reeb_menu_create(MenuCallback callback, void *context) {
    ReebMenu *owner = [ReebMenu new];
    owner.callback = callback;
    owner.context = context;
    NSString *icon = [NSBundle.mainBundle pathForResource:@"reeb" ofType:@"icns"];
    if (icon) NSApp.applicationIconImage = [[NSImage alloc] initWithContentsOfFile:icon];
    current_menu = owner;
    Class delegate = object_getClass(NSApp.delegate);
    SEL selector = @selector(applicationShouldHandleReopen:hasVisibleWindows:);
    IMP implementation = class_getMethodImplementation(delegate, selector);
    if ([NSApp.delegate respondsToSelector:selector] && implementation != (IMP)reopen) {
        previous_reopen = (BOOL (*)(id, SEL, NSApplication *, BOOL))implementation;
    }
    char signature[32];
    snprintf(signature, sizeof(signature), "%s@:@%s", @encode(BOOL), @encode(BOOL));
    if (!class_addMethod(delegate, selector, (IMP)reopen, signature)) {
        class_replaceMethod(delegate, selector, (IMP)reopen, signature);
    }
    owner.previousMenu = NSApp.mainMenu;
    NSMenu *main = [NSMenu new];
    NSMenuItem *appItem = [NSMenuItem new];
    appItem.title = @"reeb";
    [main addItem:appItem];
    NSMenu *app = [[NSMenu alloc] initWithTitle:@"reeb"];
    appItem.submenu = app;
    NSMenuItem *about = [[NSMenuItem alloc] initWithTitle:@"About reeb"
        action:@selector(orderFrontStandardAboutPanel:) keyEquivalent:@""];
    [app addItem:about];
    [app addItem:NSMenuItem.separatorItem];
    [app addItem:item(owner, @"Settings…", 1, @",")];
    [app addItem:NSMenuItem.separatorItem];
    NSMenuItem *hide = [[NSMenuItem alloc] initWithTitle:@"Hide reeb" action:@selector(hide:) keyEquivalent:@"h"];
    [app addItem:hide];
    NSMenuItem *others = [[NSMenuItem alloc] initWithTitle:@"Hide Others" action:@selector(hideOtherApplications:) keyEquivalent:@"h"];
    others.keyEquivalentModifierMask = NSEventModifierFlagCommand | NSEventModifierFlagOption;
    [app addItem:others];
    [app addItem:[[NSMenuItem alloc] initWithTitle:@"Show All" action:@selector(unhideAllApplications:) keyEquivalent:@""]];
    [app addItem:NSMenuItem.separatorItem];
    [app addItem:item(owner, @"Quit reeb", 3, @"q")];
    NSMenuItem *windowItem = [NSMenuItem new];
    windowItem.title = @"Window";
    NSMenu *window = [[NSMenu alloc] initWithTitle:@"Window"];
    windowItem.submenu = window;
    [main addItem:windowItem];
    [window addItem:item(owner, @"Show reeb", 0, @"")];
    [window addItem:[[NSMenuItem alloc] initWithTitle:@"Minimize" action:@selector(performMiniaturize:) keyEquivalent:@"m"]];
    [window addItem:item(owner, @"Close Window", 4, @"w")];
    NSApp.mainMenu = main;
    NSApp.windowsMenu = window;
    __weak ReebMenu *weakOwner = owner;
    owner.launchObserver = [NSNotificationCenter.defaultCenter
        addObserverForName:NSApplicationDidFinishLaunchingNotification object:NSApp
        queue:NSOperationQueue.mainQueue usingBlock:^(NSNotification *notification) {
            (void)notification;
            [weakOwner installStatusItem];
        }];
    [owner installStatusItem];
    return (__bridge_retained void *)owner;
}

void reeb_menu_update(void *ptr, bool enabled) {
    ReebMenu *owner = (__bridge ReebMenu *)ptr;
    owner.enabled = enabled;
    [owner installStatusItem];
    owner.toggle.title = enabled ? @"Pause Keyboard Sounds" : @"Resume Keyboard Sounds";
    owner.status.button.toolTip = enabled ? @"reeb — Keyboard sounds on" : @"reeb — Paused";
}

bool reeb_menu_available(void *ptr) {
    ReebMenu *owner = (__bridge ReebMenu *)ptr;
    [owner installStatusItem];
    return owner.status.visible && owner.status.button.image != nil;
}

void reeb_window_show(void) {
    for (NSWindow *window in NSApp.windows) {
        if (window.canBecomeMainWindow) {
            [window deminiaturize:nil];
            [window makeKeyAndOrderFront:nil];
            break;
        }
    }
    [NSApp activateIgnoringOtherApps:YES];
}

void reeb_menu_destroy(void *ptr) {
    ReebMenu *owner = (__bridge_transfer ReebMenu *)ptr;
    current_menu = nil;
    if (owner.launchObserver) [NSNotificationCenter.defaultCenter removeObserver:owner.launchObserver];
    if (previous_reopen) {
        class_replaceMethod(object_getClass(NSApp.delegate),
            @selector(applicationShouldHandleReopen:hasVisibleWindows:), (IMP)previous_reopen, NULL);
    }
    [NSStatusBar.systemStatusBar removeStatusItem:owner.status];
    NSApp.windowsMenu = nil;
    NSApp.mainMenu = owner.previousMenu;
}
