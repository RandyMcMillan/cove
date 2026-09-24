//
//  LocalNodeSettingsView.swift
//  Cove
//

import SwiftUI

struct LocalNodeSettingsView: View {
    @Environment(AppManager.self) private var app

    @State private var isRunning = false
    @State private var endpointsReady = false
    @State private var tipHeight: UInt32?
    @State private var isInIbd: Bool?
    @State private var datadirSize: UInt64?
    @State private var peerCount: UInt32?
    @State private var electrumUrl: String?
    @State private var esploraUrl: String?
    @State private var errorMessage: String?
    @State private var showClearConfirm = false
    @State private var timer: Timer? = nil
    @State private var logTimer: Timer? = nil
    @State private var isNetworkConnected = true

    // peers panel (macOS only)
    @State private var showPeersPanel = false
    @State private var peersPanelWidthRatio: CGFloat = 0.66

    // logging
    @State private var logLines: [String] = []
    @State private var logLevel: String = "trace"
    let logLevels = ["error", "warn", "info", "debug", "trace"]

    private var isMac: Bool {
        ProcessInfo.processInfo.isMacCatalystApp
    }

    private var horizonFromLogs: UInt32? {
        for line in logLines.reversed() {
            if let range = line.range(of: "horizon=") {
                let after = line[range.upperBound...]
                let number = after.prefix(while: { $0.isNumber })
                if let value = UInt32(number) {
                    return value
                }
            }
        }
        return nil
    }

    private var filteredLogLines: [String] {
        let selectedLevel = logLevelFor(logLevel)
        return logLines.filter { line in
            let lineLevel = logLevelFor(line)
            return lineLevel <= selectedLevel
        }
    }

    private func logLevelFor(_ source: String) -> Int {
        let upper = source.uppercased()
        if upper.hasPrefix("TRACE") { return 5 }
        if upper.hasPrefix("DEBUG") { return 4 }
        if upper.hasPrefix("INFO") { return 3 }
        if upper.hasPrefix("WARN") { return 2 }
        if upper.hasPrefix("ERROR") { return 1 }
        return 3
    }

    var body: some View {
        LocalNodeSettingsBody(
            isRunning: isRunning,
            endpointsReady: endpointsReady,
            tipHeight: tipHeight,
            isInIbd: isInIbd,
            datadirSize: datadirSize,
            peerCount: peerCount,
            electrumUrl: electrumUrl,
            esploraUrl: esploraUrl,
            horizonFromLogs: horizonFromLogs,
            isNetworkConnected: isNetworkConnected,
            showClearConfirm: $showClearConfirm,
            logLines: filteredLogLines,
            logLevel: $logLevel,
            onSetLogLevel: setLogLevel,
            startNode: startNode,
            stopNode: stopNode,
            showPeersPanel: $showPeersPanel,
            peersPanelWidthRatio: $peersPanelWidthRatio,
            errorMessage: $errorMessage,
            isMac: isMac,
            startPolling: startPolling,
            stopPolling: stopPolling,
            clearDatadir: clearDatadir,
            localNodeSetLogLevel: localNodeSetLogLevel,
            refreshStatus: refreshStatus,
            refreshLogs: refreshLogs
        )
        .alert("Error", isPresented: .constant(errorMessage != nil)) {
            Button("OK") { errorMessage = nil }
        } message: {
            if let errorMessage {
                Text(errorMessage)
            }
        }
        .alert("Clear Data Directory?", isPresented: $showClearConfirm) {
            Button("Cancel", role: .cancel) {}
            Button("Clear", role: .destructive) { clearDatadir() }
        } message: {
            Text("This will delete all local node data and require a full resync.")
        }
    }

    private func startPolling() {
        let interval = (isRunning && isInIbd == true) ? 0.5 : 1.0
        timer = Timer.scheduledTimer(withTimeInterval: interval, repeats: true) { _ in
            Task { await refreshStatus() }
        }

        logTimer = Timer.scheduledTimer(withTimeInterval: 2.0, repeats: true) { _ in
            Task { await refreshLogs() }
        }
    }

    private func stopPolling() {
        timer?.invalidate()
        timer = nil
        logTimer?.invalidate()
        logTimer = nil
    }

    private func restartPolling() {
        stopPolling()
        startPolling()
    }

    private func refreshStatus() async {
        do {
            let newRunning = await localNodeIsRunning()
            let newEndpointsReady = await localNodeEndpointsReady()
            let newTip = await localNodeTipHeight()
            let newIbd = await localNodeIsInIbd()
            let newPeers = await localNodePeerCount()
            let newSize = try await localNodeDatadirSize()
            let newNetworkConnected = CloudConnectivityMonitor.shared.isConnected()
            let newElectrumUrl = newEndpointsReady ? await localNodeElectrumUrl() : nil
            let newEsploraUrl = newEndpointsReady ? await localNodeEsploraUrl() : nil

            await MainActor.run {
                let wasRunning = isRunning
                let wasIbd = isInIbd

                isRunning = newRunning
                endpointsReady = newEndpointsReady
                tipHeight = newTip
                isInIbd = newIbd
                peerCount = newPeers
                datadirSize = newSize
                electrumUrl = newElectrumUrl
                esploraUrl = newEsploraUrl
                isNetworkConnected = newNetworkConnected

                if wasRunning != isRunning || wasIbd != isInIbd {
                    restartPolling()
                }
            }
        } catch {
            await MainActor.run { datadirSize = nil }
        }
    }

    private func refreshLogs() async {
        let logs = await localNodeLogs(limit: 0)
        await MainActor.run { logLines = logs }
    }

    private func startNode() {
        Task {
            do {
                try await localNodeStart(network: app.selectedNetwork)
                await refreshStatus()
                await refreshLogs()
            } catch {
                await MainActor.run { errorMessage = error.localizedDescription }
            }
        }
    }

    private func stopNode() {
        Task {
            await localNodeStop()
            await refreshStatus()
            await refreshLogs()
        }
    }

    private func clearDatadir() {
        Task {
            do {
                try await localNodeClearDatadir()
                await refreshStatus()
                await refreshLogs()
            } catch {
                await MainActor.run { errorMessage = error.localizedDescription }
            }
        }
    }

    private func setLogLevel(_ level: String) {
        logLevel = level
        Task {
            let ok = await localNodeSetLogLevel(level: level)
            if !ok {
                await MainActor.run { errorMessage = "Unknown log level: \(level)" }
            }
        }
    }
}

struct LocalNodeStatusSection: View {
    let isRunning: Bool
    let endpointsReady: Bool
    let tipHeight: UInt32?
    let isInIbd: Bool?
    let datadirSize: UInt64?
    let peerCount: UInt32?
    let electrumUrl: String?
    let esploraUrl: String?
    let horizon: UInt32?
    let isNetworkConnected: Bool

    var body: some View {
        VStack(alignment: .leading, spacing: 12) {
            Text("Status")
                .font(.subheadline)
                .fontWeight(.semibold)
                .foregroundStyle(.secondary)

            VStack(spacing: 0) {
                if isRunning, !endpointsReady {
                    StatusRow(title: "State", value: "Starting…")
                } else {
                    StatusRow(title: "State", value: isRunning ? "Running" : "Stopped")
                }

                Divider()
                NetworkStatusRow(isConnected: isNetworkConnected)

                if let peerCount {
                    Divider()
                    StatusRow(title: "Peers", value: "\(peerCount)")
                }

                if let tipHeight {
                    Divider()
                    if let horizon {
                        StatusRow(title: "Block Height", value: "\(tipHeight) / \(horizon)")
                    } else {
                        StatusRow(title: "Block Height", value: "\(tipHeight)")
                    }
                }

                if let isInIbd {
                    Divider()
                    StatusRow(title: "Initial Block Download", value: isInIbd ? "Yes" : "No")
                }

                if let datadirSize {
                    Divider()
                    StatusRow(title: "Data Directory", value: formatBytes(datadirSize))
                }

                if let electrumUrl {
                    Divider()
                    StatusRow(title: "Electrum", value: electrumUrl)
                }

                if let esploraUrl {
                    Divider()
                    StatusRow(title: "Esplora", value: esploraUrl)
                }
            }
            .padding(.vertical, 4)
            .background(Color(.secondarySystemGroupedBackground))
            .cornerRadius(10)
        }
    }
}

private struct StatusRow: View {
    let title: String
    let value: String

    var body: some View {
        HStack {
            Text(title)
            Spacer()
            Text(value)
                .foregroundStyle(.secondary)
                .font(.subheadline)
        }
        .padding(.horizontal, 16)
        .padding(.vertical, 10)
    }
}

private struct NetworkStatusRow: View {
    let isConnected: Bool

    var body: some View {
        HStack {
            Text("Network")
            Spacer()
            HStack(spacing: 6) {
                Circle()
                    .fill(isConnected ? Color.green : Color.red)
                    .frame(width: 8, height: 8)
                Text(isConnected ? "Online" : "Offline")
                    .foregroundStyle(.secondary)
                    .font(.subheadline)
            }
        }
        .padding(.horizontal, 16)
        .padding(.vertical, 10)
    }
}

struct LocalNodeActionsSection: View {
    let isRunning: Bool
    let onStart: () -> Void
    let onStop: () -> Void
    let onClear: () -> Void

    var body: some View {
        VStack(alignment: .leading, spacing: 12) {
            Text("Actions")
                .font(.subheadline)
                .fontWeight(.semibold)
                .foregroundStyle(.secondary)

            VStack(spacing: 0) {
                if isRunning {
                    Button("Stop Node", role: .destructive, action: onStop)
                        .frame(maxWidth: .infinity, alignment: .leading)
                        .padding(.horizontal, 16)
                        .padding(.vertical, 12)
                } else {
                    Button("Start Node", action: onStart)
                        .frame(maxWidth: .infinity, alignment: .leading)
                        .padding(.horizontal, 16)
                        .padding(.vertical, 12)
                }

                Divider()

                Button("Clear Data Directory", role: .destructive, action: onClear)
                    .frame(maxWidth: .infinity, alignment: .leading)
                    .padding(.horizontal, 16)
                    .padding(.vertical, 12)
                    .disabled(isRunning)
            }
            .background(Color(.secondarySystemGroupedBackground))
            .cornerRadius(10)
        }
    }
}

struct LocalNodeConnectionSection: View {
    let electrumUrl: String?
    let esploraUrl: String?

    private var electrumHostPort: String {
        electrumUrl?.replacingOccurrences(of: "tcp://", with: "") ?? "127.0.0.1:<port>"
    }

    private var electrumHost: String {
        electrumHostPort.components(separatedBy: ":").first ?? "127.0.0.1"
    }

    private var electrumPort: String {
        electrumHostPort.components(separatedBy: ":").last ?? "<port>"
    }

    var body: some View {
        VStack(alignment: .leading, spacing: 12) {
            Text("Connect")
                .font(.subheadline)
                .fontWeight(.semibold)
                .foregroundStyle(.secondary)

            VStack(alignment: .leading, spacing: 12) {
                if let esploraUrl {
                    VStack(alignment: .leading, spacing: 6) {
                        Text("Esplora (HTTP)")
                            .font(.caption)
                            .foregroundStyle(.secondary)

                        CodeBlock(
                            title: "Block height",
                            code: "curl \(esploraUrl)/blocks/tip/height"
                        )
                        CodeBlock(
                            title: "Address info",
                            code: "curl \(esploraUrl)/address/<address>"
                        )
                    }
                }

                if let electrumUrl {
                    VStack(alignment: .leading, spacing: 6) {
                        Text("Electrum (TCP)")
                            .font(.caption)
                            .foregroundStyle(.secondary)

                        CodeBlock(
                            title: "Server version",
                            code: "printf '{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"server.version\",\"params\":[\"cove\",\"1.4\"]}\\n' | nc \(electrumHost) \(electrumPort)"
                        )
                    }
                }
            }
            .padding(.horizontal, 16)
            .padding(.vertical, 12)
            .background(Color(.secondarySystemGroupedBackground))
            .cornerRadius(10)
        }
    }
}

struct LocalNodeLogSection: View {
    let logLines: [String]
    @Binding var logLevel: String
    let onSetLogLevel: (String) -> Void

    var body: some View {
        VStack(alignment: .leading, spacing: 12) {
            HStack {
                Text("Console")
                    .font(.subheadline)
                    .fontWeight(.semibold)
                    .foregroundStyle(.secondary)

                Spacer()

                LogLevelPillBox(selected: $logLevel, onSelect: onSetLogLevel)
            }
            .padding(.horizontal, 16)

            if logLines.isEmpty {
                Text("No logs yet…")
                    .font(.caption)
                    .foregroundStyle(.secondary)
                    .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .center)
            } else {
                LogConsoleView(lines: logLines)
                    .frame(maxWidth: .infinity, maxHeight: .infinity)
            }
        }
        .padding(.horizontal, 16)
        .padding(.bottom, 16)
        .frame(maxHeight: .infinity)
    }
}

private struct LogLevelPillBox: View {
    @Binding var selected: String
    let onSelect: (String) -> Void

    private let levels = ["error", "warn", "info", "debug", "trace"]

    var body: some View {
        HStack(spacing: 4) {
            ForEach(levels, id: \.self) { level in
                Button {
                    onSelect(level)
                } label: {
                    Text(level.uppercased())
                        .font(.caption2)
                        .fontWeight(.semibold)
                        .foregroundStyle(isSelected(level) ? .white : .primary)
                        .padding(.horizontal, 10)
                        .padding(.vertical, 5)
                        .frame(maxWidth: .infinity)
                }
                .buttonStyle(.plain)
                .background(isSelected(level) ? Color.blue : Color.clear)
                .clipShape(Capsule())
            }
        }
        .padding(4)
        .background(Color(.systemGray5))
        .clipShape(Capsule())
    }

    private func isSelected(_ level: String) -> Bool {
        selected == level
    }
}

private struct LogConsoleView: View {
    let lines: [String]

    var body: some View {
        ScrollViewReader { proxy in
            ScrollView {
                VStack(alignment: .leading, spacing: 2) {
                    ForEach(Array(lines.enumerated()), id: \.offset) { _, line in
                        Text(line)
                            .font(.system(size: 11, design: .monospaced))
                            .foregroundStyle(logColor(for: line))
                    }
                }
                .padding(8)
                .frame(maxWidth: .infinity, alignment: .leading)
                .textSelection(.enabled)
            }
            .background(Color(.systemGray6))
            .cornerRadius(10)
            .scrollIndicators(.hidden)
            .onChange(of: lines.count) { _, _ in
                if let last = lines.indices.last {
                    withAnimation {
                        proxy.scrollTo(last, anchor: .bottom)
                    }
                }
            }
        }
    }

    private func logColor(for line: String) -> Color {
        if line.hasPrefix("ERROR") { return .red }
        if line.hasPrefix("WARN") { return .orange }
        if line.hasPrefix("INFO") { return .primary }
        if line.hasPrefix("DEBUG") { return .secondary }
        if line.hasPrefix("TRACE") { return .gray }
        return .primary
    }
}

private func formatBytes(_ bytes: UInt64) -> String {
    let formatter = ByteCountFormatter()
    formatter.countStyle = .file
    return formatter.string(fromByteCount: Int64(bytes))
}

struct LocalNodeHelpView: View {
    let electrumUrl: String?
    let esploraUrl: String?
    @Environment(\.dismiss) private var dismiss

    private var electrumHostPort: String {
        electrumUrl?.replacingOccurrences(of: "tcp://", with: "") ?? "127.0.0.1:<port>"
    }

    private var electrumHost: String {
        electrumHostPort.components(separatedBy: ":").first ?? "127.0.0.1"
    }

    private var electrumPort: String {
        electrumHostPort.components(separatedBy: ":").last ?? "<port>"
    }

    var body: some View {
        NavigationStack {
            ScrollView {
                VStack(alignment: .leading, spacing: 20) {
                    Text("The local node exposes an Electrum (TCP) and an Esplora (HTTP) endpoint on localhost. Ports change every time the node restarts.")
                        .font(.body)

                    helpSection("Esplora (HTTP)") {
                        Text("The Esplora endpoint speaks plain HTTP and can be queried with curl, wget, or any HTTP client.")
                            .font(.subheadline)
                            .foregroundStyle(.secondary)

                        CodeBlock(
                            title: "Block height",
                            code: "curl \(esploraUrl ?? "http://127.0.0.1:<port>")/blocks/tip/height"
                        )
                        CodeBlock(
                            title: "Address info",
                            code: "curl \(esploraUrl ?? "http://127.0.0.1:<port>")/address/<address>"
                        )
                        CodeBlock(
                            title: "Address transactions",
                            code: "curl \(esploraUrl ?? "http://127.0.0.1:<port>")/address/<address>/txs"
                        )
                        CodeBlock(
                            title: "Address UTXOs",
                            code: "curl \(esploraUrl ?? "http://127.0.0.1:<port>")/address/<address>/utxo"
                        )
                        CodeBlock(
                            title: "Transaction by ID",
                            code: "curl \(esploraUrl ?? "http://127.0.0.1:<port>")/tx/<txid>"
                        )
                        CodeBlock(
                            title: "Broadcast raw transaction",
                            code: "curl -X POST -H \"Content-Type: text/plain\" --data-binary \"<hex-encoded-tx>\" \(esploraUrl ?? "http://127.0.0.1:<port>")/tx"
                        )
                    }

                    helpSection("Electrum (TCP)") {
                        Text("The Electrum endpoint speaks the Electrum protocol over plain TCP. Use nc (netcat) or a dedicated client.")
                            .font(.subheadline)
                            .foregroundStyle(.secondary)

                        CodeBlock(
                            title: "Server version",
                            code: "printf '{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"server.version\",\"params\":[\"cove\",\"1.4\"]}\\n' | nc \(electrumHost) \(electrumPort)"
                        )
                        CodeBlock(
                            title: "Block headers",
                            code: "printf '{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"blockchain.headers.subscribe\",\"params\":[]}\\n' | nc \(electrumHost) \(electrumPort)"
                        )
                        CodeBlock(
                            title: "Scripthash balance",
                            code: "printf '{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"blockchain.scripthash.get_balance\",\"params\":[\"<scripthash>\"]}\\n' | nc \(electrumHost) \(electrumPort)"
                        )
                        CodeBlock(
                            title: "Scripthash history",
                            code: "printf '{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"blockchain.scripthash.get_history\",\"params\":[\"<scripthash>\"]}\\n' | nc \(electrumHost) \(electrumPort)"
                        )
                    }

                    helpSection("Notes") {
                        VStack(alignment: .leading, spacing: 8) {
                            Label("Endpoints appear only after IBD and index materialization complete — the Status section shows 'Starting…' until then.", systemImage: "info.circle")
                            Label("Ports change on every restart — check the Status section for current URLs.", systemImage: "info.circle")
                            Label("The node only listens on 127.0.0.1 (localhost).", systemImage: "lock.shield")
                            Label("Queries may return stale data while Initial Block Download is in progress.", systemImage: "exclamationmark.triangle")
                            Label("If running in the iOS Simulator, run these commands from inside the simulator (xcrun simctl spawn <UDID> curl ...).", systemImage: "exclamationmark.triangle")
                                .foregroundStyle(.orange)
                        }
                        .font(.subheadline)
                        .foregroundStyle(.secondary)
                    }
                }
                .padding()
            }
            .navigationTitle("Local Node Help")
            .navigationBarTitleDisplayMode(.inline)
            .toolbar {
                ToolbarItem(placement: .topBarTrailing) {
                    Button("Done") { dismiss() }
                }
            }
        }
    }

    private func helpSection(_ title: String, @ViewBuilder content: () -> some View) -> some View {
        VStack(alignment: .leading, spacing: 12) {
            Text(title)
                .font(.headline)
            content()
        }
    }
}

private struct CodeBlock: View {
    let title: String
    let code: String

    @State private var didCopy = false

    var body: some View {
        VStack(alignment: .leading, spacing: 4) {
            Text(title)
                .font(.caption)
                .foregroundStyle(.secondary)

            HStack(spacing: 8) {
                Text(code)
                    .font(.system(size: 12, design: .monospaced))
                    .frame(maxWidth: .infinity, alignment: .leading)
                    .textSelection(.enabled)

                Button {
                    UIPasteboard.general.string = code
                    didCopy = true
                    DispatchQueue.main.asyncAfter(deadline: .now() + 1.5) {
                        didCopy = false
                    }
                } label: {
                    Image(systemName: didCopy ? "checkmark" : "doc.on.doc")
                        .font(.caption)
                        .foregroundStyle(.secondary)
                }
                .buttonStyle(.plain)
            }
            .padding(10)
            .background(Color(.systemGray6))
            .cornerRadius(8)
        }
    }
}

struct PeersPanelView: View {
    let peerCount: UInt32?
    let isRunning: Bool

    var body: some View {
        VStack(alignment: .leading, spacing: 0) {
            HStack {
                Text("Peers")
                    .font(.headline)
                Spacer()
                if let peerCount {
                    Text("\(peerCount) connected")
                        .font(.caption)
                        .foregroundStyle(.secondary)
                }
            }
            .padding()

            Divider()

            if isRunning {
                if let peerCount, peerCount > 0 {
                    Text("Peer list will be populated here as connection details are exposed from rbitcoin.")
                        .font(.caption)
                        .foregroundStyle(.secondary)
                        .padding()
                } else {
                    Text("No peers connected yet.")
                        .font(.caption)
                        .foregroundStyle(.secondary)
                        .padding()
                }
            } else {
                Text("Node is stopped.")
                    .font(.caption)
                    .foregroundStyle(.secondary)
                    .padding()
            }

            Spacer()
        }
    }
}

#Preview {
    NavigationStack {
        LocalNodeSettingsView()
            .environment(AppManager.shared)
    }
}
