//
//  LocalNodeSettingsView.swift
//  Cove
//

import SwiftUI

struct LocalNodeSettingsView: View {
    @Environment(AppManager.self) private var app

    @State private var isRunning = false
    @State private var tipHeight: UInt32?
    @State private var isInIbd: Bool?
    @State private var datadirSize: UInt64?
    @State private var errorMessage: String?
    @State private var showClearConfirm = false
    @State private var timer: Timer? = nil

    // logging
    @State private var logLines: [String] = []
    @State private var logLevel: String = "info"
    @State private var showLogLevelPicker = false

    let logLevels = ["error", "warn", "info", "debug", "trace"]

    // approximate space taken by status + actions sections + nav bar + padding
    private let fixedSectionsHeight: CGFloat = 320

    var body: some View {
        GeometryReader { geometry in
            Form {
                LocalNodeStatusSection(
                    isRunning: isRunning,
                    tipHeight: tipHeight,
                    isInIbd: isInIbd,
                    datadirSize: datadirSize
                )

                LocalNodeActionsSection(
                    isRunning: isRunning,
                    onStart: startNode,
                    onStop: stopNode,
                    onClear: { showClearConfirm = true }
                )

                LocalNodeLogSection(
                    logLines: logLines,
                    logLevel: logLevel,
                    onChangeLogLevel: { showLogLevelPicker = true },
                    maxHeight: max(120, geometry.size.height - fixedSectionsHeight)
                )
            }
            .scrollContentBackground(.hidden)
        }
        .navigationTitle("Local Node")
        .onAppear {
            Task { await refreshState() }
            startPolling()
        }
        .onDisappear {
            stopPolling()
        }
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
        .confirmationDialog("Log Level", isPresented: $showLogLevelPicker) {
            ForEach(logLevels, id: \.self) { level in
                Button(level.uppercased()) {
                    setLogLevel(level)
                }
            }
            Button("Cancel", role: .cancel) {}
        }
    }

    private func startPolling() {
        // Poll faster during IBD (0.5s) vs idle (2s)
        let interval = (isRunning && isInIbd == true) ? 0.5 : 2.0
        timer = Timer.scheduledTimer(withTimeInterval: interval, repeats: true) { _ in
            Task { await refreshState() }
        }
    }

    private func stopPolling() {
        timer?.invalidate()
        timer = nil
    }

    private func restartPolling() {
        stopPolling()
        startPolling()
    }

    private func refreshState() async {
        do {
            let wasRunning = isRunning
            let wasIbd = isInIbd
            isRunning = await localNodeIsRunning()
            tipHeight = await localNodeTipHeight()
            isInIbd = await localNodeIsInIbd()
            datadirSize = try await localNodeDatadirSize()

            // Adjust polling speed when IBD state changes
            if wasRunning != isRunning || wasIbd != isInIbd {
                await MainActor.run { restartPolling() }
            }

            // Fetch logs
            let logs = await localNodeLogs(limit: 100)
            await MainActor.run {
                logLines = logs
            }
        } catch {
            datadirSize = nil
        }
    }

    private func startNode() {
        Task {
            do {
                try await localNodeStart(network: app.selectedNetwork)
                await refreshState()
            } catch {
                await MainActor.run { errorMessage = error.localizedDescription }
            }
        }
    }

    private func stopNode() {
        Task {
            await localNodeStop()
            await refreshState()
        }
    }

    private func clearDatadir() {
        Task {
            do {
                try await localNodeClearDatadir()
                await refreshState()
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

private struct LocalNodeStatusSection: View {
    let isRunning: Bool
    let tipHeight: UInt32?
    let isInIbd: Bool?
    let datadirSize: UInt64?

    var body: some View {
        Section("Status") {
            StatusRow(title: "State", value: isRunning ? "Running" : "Stopped")

            if let tipHeight {
                StatusRow(title: "Block Height", value: "\(tipHeight)")
            }

            if let isInIbd {
                StatusRow(title: "Initial Block Download", value: isInIbd ? "Yes" : "No")
            }

            if let datadirSize {
                StatusRow(title: "Data Directory", value: formatBytes(datadirSize))
            }
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
    }
}

private struct LocalNodeActionsSection: View {
    let isRunning: Bool
    let onStart: () -> Void
    let onStop: () -> Void
    let onClear: () -> Void

    var body: some View {
        Section("Actions") {
            if isRunning {
                Button("Stop Node", role: .destructive, action: onStop)
            } else {
                Button("Start Node", action: onStart)
            }

            Button("Clear Data Directory", role: .destructive, action: onClear)
                .disabled(isRunning)
        }
    }
}

private struct LocalNodeLogSection: View {
    let logLines: [String]
    let logLevel: String
    let onChangeLogLevel: () -> Void
    let maxHeight: CGFloat

    var body: some View {
        Section {
            HStack {
                Text("Log Level")
                Spacer()
                Button(logLevel.uppercased(), action: onChangeLogLevel)
                    .font(.subheadline)
                    .foregroundStyle(.blue)
            }

            if logLines.isEmpty {
                Text("No logs yet…")
                    .font(.caption)
                    .foregroundStyle(.secondary)
                    .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .center)
                    .padding(.vertical, 8)
            } else {
                LogConsoleView(lines: logLines)
                    .frame(maxWidth: .infinity, maxHeight: maxHeight)
            }
        } header: {
            Text("Console")
        }
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
                            .textSelection(.enabled)
                    }
                }
                .padding(8)
                .frame(maxWidth: .infinity, alignment: .leading)
            }
            .background(Color(.systemGray6))
            .cornerRadius(8)
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

#Preview {
    NavigationStack {
        LocalNodeSettingsView()
            .environment(AppManager.shared)
    }
}
