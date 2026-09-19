//
//  LocalNodeSettingsView.swift
//  Cove
//

import SwiftUI

struct LocalNodeSettingsView: View {
    @Environment(AppManager.self) private var app
    @Environment(\.colorScheme) private var colorScheme

    @State private var isRunning = false
    @State private var tipHeight: UInt32?
    @State private var isInIbd: Bool?
    @State private var datadirSize: UInt64?
    @State private var errorMessage: String?
    @State private var showClearConfirm = false
    @State private var timer: Timer? = nil

    var body: some View {
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
        }
        .scrollContentBackground(.hidden)
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
        timer = Timer.scheduledTimer(withTimeInterval: 2.0, repeats: true) { _ in
            Task { await refreshState() }
        }
    }

    private func stopPolling() {
        timer?.invalidate()
        timer = nil
    }

    private func refreshState() async {
        do {
            isRunning = await localNodeIsRunning()
            tipHeight = await localNodeTipHeight()
            isInIbd = await localNodeIsInIbd()
            datadirSize = try await localNodeDatadirSize()
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
