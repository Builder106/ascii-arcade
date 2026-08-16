import WidgetKit
import SwiftUI

struct Provider: TimelineProvider {
    func placeholder(in context: Context) -> SimpleEntry {
        SimpleEntry(date: Date(), glyph: "A")
    }

    func getSnapshot(in context: Context, completion: @escaping (SimpleEntry) -> Void) {
        let entry = SimpleEntry(date: Date(), glyph: "A")
        completion(entry)
    }

    func getTimeline(in context: Context, completion: @escaping (Timeline<SimpleEntry>) -> Void) {
        // Refresh every 15 minutes (WidgetKit's practical lower limit)
        let refreshDate = Calendar.current.date(byAdding: .minute, value: 15, to: Date())!
        
        // In a real implementation, we would query the AaEngine for a single character frame
        // or a small grid, but since AaEngine requires metal/MTKView context or a decoded buffer,
        // we simulate a single character for the circular widget, or a mini grid for rectangular.
        
        let entry = SimpleEntry(date: Date(), glyph: "█")
        let timeline = Timeline(entries: [entry], policy: .after(refreshDate))
        completion(timeline)
    }
}

struct SimpleEntry: TimelineEntry {
    let date: Date
    let glyph: String
}

struct AsciiArcadeWidgetEntryView : View {
    var entry: Provider.Entry
    @Environment(\.widgetFamily) var family

    var body: some View {
        switch family {
        case .accessoryCircular:
            ZStack {
                Circle().fill(Color.black)
                Text(entry.glyph)
                    .font(.system(size: 24, design: .monospaced))
                    .foregroundColor(.green)
            }
        case .accessoryRectangular:
            VStack(alignment: .leading, spacing: 2) {
                Text("ASCII ARCADE")
                    .font(.system(size: 10, design: .monospaced))
                    .foregroundColor(.green)
                Text(String(repeating: entry.glyph, count: 10))
                    .font(.system(size: 14, design: .monospaced))
                    .foregroundColor(.green)
                Text(String(repeating: entry.glyph, count: 10))
                    .font(.system(size: 14, design: .monospaced))
                    .foregroundColor(.green)
            }
            .frame(maxWidth: .infinity, maxHeight: .infinity)
            .background(Color.black)
        default:
            Text("Unsupported")
        }
    }
}

@main
struct AsciiArcadeWidget: Widget {
    let kind: String = "AsciiArcadeWidget"

    var body: some WidgetConfiguration {
        StaticConfiguration(kind: kind, provider: Provider()) { entry in
            AsciiArcadeWidgetEntryView(entry: entry)
        }
        .configurationDisplayName("ASCII Arcade")
        .description("Animated ASCII scenes on your lock screen.")
        .supportedFamilies([.accessoryCircular, .accessoryRectangular])
    }
}
