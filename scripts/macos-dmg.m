#import <AppKit/AppKit.h>
#import <CoreServices/CoreServices.h>

static void text(NSString *value, NSRect rect, CGFloat size, NSFontWeight weight, NSColor *color) {
    NSMutableParagraphStyle *paragraph = [NSMutableParagraphStyle new];
    paragraph.alignment = NSTextAlignmentCenter;
    [value drawInRect:rect withAttributes:@{NSFontAttributeName:[NSFont systemFontOfSize:size weight:weight],
        NSForegroundColorAttributeName:color, NSParagraphStyleAttributeName:paragraph}];
}

static int create_assets(NSString *logoPath, NSString *iconPath, NSString *stage) {
    NSFileManager *files = NSFileManager.defaultManager;
    NSString *hidden = [stage stringByAppendingPathComponent:@".background"];
    NSError *error = nil;
    if (![files createDirectoryAtPath:hidden withIntermediateDirectories:YES attributes:nil error:&error]) {
        NSLog(@"%@", error); return 1;
    }
    NSBitmapImageRep *bitmap = [[NSBitmapImageRep alloc] initWithBitmapDataPlanes:NULL
        pixelsWide:1280 pixelsHigh:800 bitsPerSample:8 samplesPerPixel:4 hasAlpha:YES
        isPlanar:NO colorSpaceName:NSDeviceRGBColorSpace bytesPerRow:0 bitsPerPixel:0];
    NSGraphicsContext *context = [NSGraphicsContext graphicsContextWithBitmapImageRep:bitmap];
    [NSGraphicsContext saveGraphicsState];
    NSGraphicsContext.currentContext = context;
    CGContextScaleCTM(context.CGContext, 2, 2);
    [[NSColor colorWithCalibratedRed:0.96 green:0.97 blue:0.99 alpha:1] setFill];
    NSRectFill(NSMakeRect(0, 0, 640, 400));
    NSImage *logo = [[NSImage alloc] initWithContentsOfFile:logoPath];
    if (!logo) return 1;
    [logo drawInRect:NSMakeRect(260, 321, 36, 36) fromRect:NSZeroRect operation:NSCompositingOperationSourceOver fraction:1];
    NSColor *ink = [NSColor colorWithCalibratedRed:0.11 green:0.14 blue:0.20 alpha:1];
    text(@"reeb", NSMakeRect(300, 317, 82, 43), 32, NSFontWeightSemibold, ink);
    text(@"drag reeb to the folder", NSMakeRect(80, 271, 480, 28), 18, NSFontWeightMedium, ink);
    NSBezierPath *arrow = [NSBezierPath bezierPath];
    arrow.lineWidth = 2.5;
    arrow.lineCapStyle = NSLineCapStyleRound;
    [arrow moveToPoint:NSMakePoint(290, 184)];
    [arrow lineToPoint:NSMakePoint(350, 184)];
    [arrow moveToPoint:NSMakePoint(338, 196)];
    [arrow lineToPoint:NSMakePoint(350, 184)];
    [arrow lineToPoint:NSMakePoint(338, 172)];
    [[NSColor colorWithCalibratedRed:0.49 green:0.55 blue:0.66 alpha:1] setStroke];
    [arrow stroke];
    text(@"then open reeb from applications", NSMakeRect(60, 40, 520, 24), 13, NSFontWeightRegular,
        [NSColor colorWithCalibratedRed:0.40 green:0.45 blue:0.54 alpha:1]);
    [NSGraphicsContext restoreGraphicsState];
    bitmap.size = NSMakeSize(640, 400);
    NSData *png = [bitmap representationUsingType:NSBitmapImageFileTypePNG properties:@{}];
    if (![png writeToFile:[hidden stringByAppendingPathComponent:@"background.png"] options:NSDataWritingAtomic error:&error]) {
        NSLog(@"%@", error); return 1;
    }
    if (![files copyItemAtPath:iconPath toPath:[stage stringByAppendingPathComponent:@".VolumeIcon.icns"] error:&error]) {
        NSLog(@"%@", error); return 1;
    }
    return 0;
}

static int create_alias(NSString *mount, NSString *output) {
    // Finder's icvp background field requires a Carbon Alias record.
    // This deprecated API is confined to the packaging tool.
    NSString *picture = [mount stringByAppendingPathComponent:@".background/background.png"];
    FSRef folder, target;
    if (FSPathMakeRef((const UInt8 *)mount.fileSystemRepresentation, &folder, NULL) != noErr ||
        FSPathMakeRef((const UInt8 *)picture.fileSystemRepresentation, &target, NULL) != noErr) return 1;
    AliasHandle alias = NULL;
    if (FSNewAlias(&folder, &target, &alias) != noErr) return 1;
    NSData *data = [NSData dataWithBytes:*alias length:GetAliasSize(alias)];
    BOOL success = [data writeToFile:output atomically:YES];
    DisposeHandle((Handle)alias);
    return success ? 0 : 1;
}

int main(int argc, const char *argv[]) {
    @autoreleasepool {
        if (argc == 5 && strcmp(argv[1], "assets") == 0) {
            return create_assets(@(argv[2]), @(argv[3]), @(argv[4]));
        }
        if (argc == 4 && strcmp(argv[1], "alias") == 0) {
            return create_alias(@(argv[2]), @(argv[3]));
        }
        fprintf(stderr, "Usage: macos-dmg assets logo.png icon.icns stage | alias mount output\n");
        return 2;
    }
}
