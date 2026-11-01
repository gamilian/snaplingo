#import <Foundation/Foundation.h>
#import <Vision/Vision.h>

static void emit(NSFileHandle *output, NSDictionary *row) {
    NSData *data = [NSJSONSerialization dataWithJSONObject:row options:0 error:nil];
    [output writeData:data];
    [output writeData:[@"\n" dataUsingEncoding:NSUTF8StringEncoding]];
}

int main(int argc, const char **argv) {
    @autoreleasepool {
        if (argc < 4) {
            fprintf(stderr, "usage: vision-bench MANIFEST OUTPUT REPEATS [cpu]\n");
            return 2;
        }
        NSString *manifestPath = [NSString stringWithUTF8String:argv[1]];
        NSString *outputPath = [NSString stringWithUTF8String:argv[2]];
        BOOL cpuOnly = argc > 4 && strcmp(argv[4], "cpu") == 0;
        int repeats = atoi(argv[3]);
        [[NSFileManager defaultManager] createFileAtPath:outputPath contents:nil attributes:nil];
        NSFileHandle *output = [NSFileHandle fileHandleForWritingAtPath:outputPath];
        NSString *manifest = [NSString stringWithContentsOfFile:manifestPath
                                                     encoding:NSUTF8StringEncoding error:nil];
        emit(output, @{@"type": @"environment", @"backend": cpuOnly ? @"vision_cpu" : @"vision_auto",
                       @"pid": @([[NSProcessInfo processInfo] processIdentifier]),
                       @"uses_cpu_only": @(cpuOnly),
                       @"compute_device_claim": cpuOnly ? @"CPU requested" : @"system-selected; no GPU attribution"});
        for (int iteration = 0; iteration <= repeats; iteration++) {
            for (NSString *line in [manifest componentsSeparatedByString:@"\n"]) {
                if (line.length == 0) continue;
                @autoreleasepool {
                    NSDictionary *row = [NSJSONSerialization JSONObjectWithData:
                        [line dataUsingEncoding:NSUTF8StringEncoding] options:0 error:nil];
                    CFAbsoluteTime start = CFAbsoluteTimeGetCurrent();
                    NSData *data = [NSData dataWithContentsOfFile:row[@"image"]];
                    double readMS = (CFAbsoluteTimeGetCurrent() - start) * 1000;
                    VNRecognizeTextRequest *request = [[VNRecognizeTextRequest alloc] init];
                    request.recognitionLevel = VNRequestTextRecognitionLevelAccurate;
                    request.usesLanguageCorrection = YES;
                    request.automaticallyDetectsLanguage = YES;
                    request.recognitionLanguages = @[@"zh-Hans", @"en-US"];
                    request.usesCPUOnly = cpuOnly;
                    VNImageRequestHandler *handler = [[VNImageRequestHandler alloc] initWithData:data options:@{}];
                    CFAbsoluteTime inferStart = CFAbsoluteTimeGetCurrent();
                    NSError *error = nil;
                    BOOL ok = [handler performRequests:@[request] error:&error];
                    double ocrMS = (CFAbsoluteTimeGetCurrent() - inferStart) * 1000;
                    NSMutableArray *texts = [NSMutableArray array];
                    for (VNRecognizedTextObservation *observation in request.results) {
                        VNRecognizedText *text = [[observation topCandidates:1] firstObject];
                        if (text) [texts addObject:text.string];
                    }
                    emit(output, @{@"type": @"image", @"id": row[@"id"], @"model": @"vision",
                                   @"backend": cpuOnly ? @"vision_cpu" : @"vision_auto",
                                   @"warmup": @(iteration == 0), @"iteration": @(iteration - 1),
                                   @"decode_ms": @(readMS), @"ocr_ms": @(ocrMS),
                                   @"total_ms": @(readMS + ocrMS), @"line_count": @(texts.count),
                                   @"text": [texts componentsJoinedByString:@"\n"],
                                   @"error": ok ? [NSNull null] : error.localizedDescription});
                }
            }
        }
        [output closeFile];
    }
    return 0;
}
