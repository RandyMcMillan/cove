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
    let logLevels = ["error", "warn", "info", "debug", "trace"]

    var body: some View {
        GeometryReader { geometry in
            VStack(spacing: 0) {
                LocalNodeStatusSection(
                    isRunning: isRunning,
                    tipHeight: tipHeight,
                    isInIbd: isInIbd,
                    datadirSize: datadirSize
                )
                .padding(.horizontal, 16)
                .padding(.top, 16)

                LocalNodeActionsSection(
                    isRunning: isRunning,
                    onStart: startNode,
                    onStop: stopNode,
                    onClear: { showClearConfirm = true }
                )
                .padding(.horizontal, 16)
                .padding(.top, 16)

                LocalNodeLogSection(
                    logLines: logLines,
                    logLevel: $logLevel,
                    onSetLogLevel: setLogLevel
                )
                .padding(.top, 16)
                .frame(maxHeight: .infinity)
            }
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
    }

    private func startPolling() {
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

            if wasRunning != isRunning || wasIbd != isInIbd {
                await MainActor.run { restartPolling() }
            }

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
        VStack(alignment: .leading, spacing: 12) {
            Text("Status")
                .font(.subheadline)
                .fontWeight(.semibold)
                .foregroundStyle(.secondary)

            VStack(spacing: 0) {
                StatusRow(title: "State", value: isRunning ? "Running" : "Stopped")

                if let tipHeight {
                    Divider()
                    StatusRow(title: "Block Height", value: "\(tipHeight)")
                }

                if let isInIbd {
                    Divider()
                    StatusRow(title: "Initial Block Download", value: isInIbd ? "Yes" : "No")
                }

                if let datadirSize {
                    Divider()
                    StatusRow(title: "Data Directory", value: formatBytes(datadirSize))
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

private struct LocalNodeActionsSection: View {
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

private struct LocalNodeLogSection: View {
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
                            .textSelection(.enabled)
                    }
                }
                .padding(8)
                .frame(maxWidth: .infinity, alignment: .leading)
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

#Preview {
    NavigationStack {
        LocalNodeSettingsView()
            .environment(AppManager.shared)
    }
}
