// Apple Silicon HID temperature reader. The access pattern follows the
// IOHIDEventSystemClient approach used by MenuMeters and exelban/Stats.
#include <CoreFoundation/CoreFoundation.h>
#include <IOKit/hidsystem/IOHIDEventSystemClient.h>
#include <math.h>
#include <string.h>

typedef struct __IOHIDEvent *IOHIDEventRef;
typedef struct __IOHIDServiceClient *IOHIDServiceClientRef;

extern IOHIDEventSystemClientRef IOHIDEventSystemClientCreate(CFAllocatorRef);
extern int IOHIDEventSystemClientSetMatching(IOHIDEventSystemClientRef, CFDictionaryRef);
extern IOHIDEventRef IOHIDServiceClientCopyEvent(IOHIDServiceClientRef, int64_t, int32_t, int64_t);
extern CFTypeRef IOHIDServiceClientCopyProperty(IOHIDServiceClientRef, CFStringRef);
extern double IOHIDEventGetFloatValue(IOHIDEventRef, int32_t);

double rldyour_gpu_hid_temperature(void) {
    int32_t page = 0xff00;
    int32_t usage = 0x0005;
    int32_t event_type = 15;
    CFNumberRef page_value = CFNumberCreate(NULL, kCFNumberSInt32Type, &page);
    CFNumberRef usage_value = CFNumberCreate(NULL, kCFNumberSInt32Type, &usage);
    const void *keys[] = { CFSTR("PrimaryUsagePage"), CFSTR("PrimaryUsage") };
    const void *values[] = { page_value, usage_value };
    CFDictionaryRef matching = CFDictionaryCreate(NULL, keys, values, 2,
        &kCFTypeDictionaryKeyCallBacks, &kCFTypeDictionaryValueCallBacks);
    IOHIDEventSystemClientRef system = IOHIDEventSystemClientCreate(NULL);
    if (!system || !matching) return NAN;
    IOHIDEventSystemClientSetMatching(system, matching);
    CFArrayRef services = IOHIDEventSystemClientCopyServices(system);
    double total = 0.0;
    size_t count = 0;
    if (services) {
        CFIndex length = CFArrayGetCount(services);
        for (CFIndex index = 0; index < length; index++) {
            IOHIDServiceClientRef service = (IOHIDServiceClientRef)CFArrayGetValueAtIndex(services, index);
            CFStringRef product = (CFStringRef)IOHIDServiceClientCopyProperty(service, CFSTR("Product"));
            char name[128] = {0};
            if (product) CFStringGetCString(product, name, sizeof(name), kCFStringEncodingUTF8);
            IOHIDEventRef event = IOHIDServiceClientCopyEvent(service, event_type, 0, 0);
            if (event && strncmp(name, "GPU MTR Temp Sensor", 19) == 0) {
                double value = IOHIDEventGetFloatValue(event, event_type << 16);
                if (isfinite(value) && value >= 10.0 && value <= 120.0) { total += value; count++; }
            }
            if (event) CFRelease(event);
            if (product) CFRelease(product);
        }
        CFRelease(services);
    }
    CFRelease(system);
    CFRelease(matching);
    CFRelease(page_value);
    CFRelease(usage_value);
    return count ? total / (double)count : NAN;
}
