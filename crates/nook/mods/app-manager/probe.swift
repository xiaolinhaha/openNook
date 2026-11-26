import AppKit
import Darwin

struct AppInfo: Encodable {
    let pid: Int32
    let pids: [Int32]
    let name: String
    let bundleId: String
    let path: String
    let policy: String // "regular" (窗口), "accessory" (菜单栏/托盘), "background" (后台服务)
    let memoryMB: Double
    let isHidden: Bool
    /// How many OS processes belong to this app. ChatGPT, Chrome and similar apps
    /// spawn helpers that all advertise the same bundle name.
    let processes: Int
}

func getMemoryMB(pid: Int32) -> Double {
    var info = proc_taskallinfo()
    let size = MemoryLayout<proc_taskallinfo>.size
    let res = proc_pidinfo(pid, PROC_PIDTASKALLINFO, 0, &info, Int32(size))
    if res == size {
        let memMB = Double(info.ptinfo.pti_resident_size) / (1024.0 * 1024.0)
        return (memMB * 10.0).rounded() / 10.0
    }
    return 0.0
}

func sameFile(_ a: URL, _ b: URL) -> Bool {
    a.resolvingSymlinksInPath().standardizedFileURL.path == b.resolvingSymlinksInPath().standardizedFileURL.path
}

/// The process whose executable is the bundle's real binary, not a renderer,
/// node worker, or helper nested inside the same .app.
func isMainProcess(_ app: NSRunningApplication) -> Bool {
    guard let bundleURL = app.bundleURL, let exec = app.executableURL else { return false }
    guard let main = Bundle(url: bundleURL)?.executableURL else { return false }
    return sameFile(exec, main)
}

/// `/Applications/ChatGPT.app/Contents/.../Helper.app` belongs to ChatGPT, not to the helper.
func hostAppPath(_ app: NSRunningApplication) -> String {
    let candidates = [app.bundleURL?.path, app.executableURL?.path].compactMap { $0 }
    for raw in candidates {
        guard let range = raw.range(of: ".app") else { continue }
        let end = raw.index(range.lowerBound, offsetBy: 4)
        if end == raw.endIndex || raw[end] == "/" {
            return String(raw[..<end])
        }
    }
    return "pid:\(app.processIdentifier)"
}

func policyName(_ app: NSRunningApplication) -> String {
    switch app.activationPolicy {
    case .regular: return "regular"
    case .accessory: return "accessory"
    default: return "background"
    }
}

func listApps() {
    let ws = NSWorkspace.shared
    struct Member {
        let app: NSRunningApplication
        let memory: Double
    }
    var groups: [String: [Member]] = [:]

    for app in ws.runningApplications {
        let path = app.bundleURL?.path ?? ""
        // Exclude system background daemons and internal XPC services
        if path.hasPrefix("/System/Library/") || path.hasPrefix("/usr/libexec/") {
            continue
        }
        guard let name = app.localizedName, !name.isEmpty else { continue }
        let bundleId = app.bundleIdentifier ?? ""

        // Skip web renderers/helpers that clutter the user app view
        if name.contains("Helper (Renderer)") || name.contains("Helper (GPU)") || name.contains("Helper (Plugin)") || name.contains("WebKit.WebContent") || bundleId.hasPrefix("com.apple.WebKit") {
            continue
        }

        let key = hostAppPath(app)
        // System daemons were already dropped; also drop helpers whose host app is in /System.
        if key.hasPrefix("/System/") || key.hasPrefix("/usr/") {
            continue
        }
        groups[key, default: []].append(Member(app: app, memory: getMemoryMB(pid: app.processIdentifier)))
    }

    var list: [AppInfo] = []
    for (key, members) in groups {
        let main = members.first { isMainProcess($0.app) }?.app
            ?? members.first { $0.app.activationPolicy == .regular }?.app
            ?? members.min { $0.app.processIdentifier < $1.app.processIdentifier }!.app
        let name = main.localizedName ?? members.first?.app.localizedName ?? key
        let path = main.bundleURL?.path ?? ""
        // A windowed process makes the whole app a window app, even when helpers are accessory.
        let policy: String
        if members.contains(where: { $0.app.activationPolicy == .regular }) {
            policy = "regular"
        } else if members.contains(where: { $0.app.activationPolicy == .accessory }) {
            policy = "accessory"
        } else {
            policy = policyName(main)
        }
        let pids = members.map { $0.app.processIdentifier }.sorted()
        let memory = (members.reduce(0.0) { $0 + $1.memory } * 10.0).rounded() / 10.0
        list.append(AppInfo(
            pid: main.processIdentifier,
            pids: pids,
            name: name,
            bundleId: main.bundleIdentifier ?? "",
            path: path,
            policy: policy,
            memoryMB: memory,
            isHidden: main.isHidden,
            processes: members.count
        ))
    }

    // Sort primarily by memory descending
    list.sort { $0.memoryMB > $1.memoryMB }

    if let data = try? JSONEncoder().encode(list),
       let jsonStr = String(data: data, encoding: .utf8) {
        print(jsonStr)
    } else {
        print("[]")
    }
}

// Handle arguments
let args = CommandLine.arguments
if args.count >= 3 && args[1] == "--kill" {
    if let pid = Int32(args[2]) {
        kill(pid, SIGKILL)
        print("{\"success\":true,\"pid\":\(pid)}")
    }
} else if args.count >= 3 && args[1] == "--term" {
    if let pid = Int32(args[2]) {
        kill(pid, SIGTERM)
        print("{\"success\":true,\"pid\":\(pid)}")
    }
} else {
    listApps()
}
