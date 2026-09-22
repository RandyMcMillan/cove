//
//  NodeSelectionView.swift
//  Cove
//
//  Created by Praveen Perera on 7/18/24.
//

import MijickPopups
import SwiftUI

struct NodeSelectionView: View {
    @Environment(AppManager.self) private var app

    /// node selection
    private let nodeSelector = NodeSelector()

    @State private var selectedNodeName: String
    @State private var nodeList: [NodeSelection]

    @State private var customNodeName: String = ""
    @State private var customUrl: String = ""

    @State private var showParseUrlAlert = false
    @State private var parseUrlMessage = ""

    @State private var checkUrlTask: Task<Void, Never>?

    /// local node
    @State private var isRunning = false
    @State private var tipHeight: UInt32?
    @State private var isInIbd: Bool?
    @State private var datadirSize: UInt64?
    @State private var peerCount: UInt32?
    @State private var errorMessage: String?
    @State private var showClearConfirm = false
    @State private var timer: Timer? = nil
    @State private var isNetworkConnected = true

    @State private var logLines: [String] = []
    @State private var logLevel: String = "trace"

    // peers panel (macOS only)
    @State private var showPeersPanel = false
    @State private var peersPanelWidthRatio: CGFloat = 0.66

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

    init() {
        selectedNodeName = NodeSelector().selectedNode().name
        nodeList = NodeSelector().nodeList()
    }

    var showCustomUrlField: Bool {
        selectedNodeName.hasPrefix("Custom")
    }

    // MARK: Node selection

    func cancelCheckUrlTask() {
        if let checkUrlTask {
            checkUrlTask.cancel()
        }
    }

    @MainActor
    private func refreshNodeState() {
        let refreshedNodeSelector = NodeSelector()
        nodeList = refreshedNodeSelector.nodeList()
        selectedNodeName = refreshedNodeSelector.selectedNode().name
    }

    private func showLoadingPopup() {
        cancelCheckUrlTask()

        Task { @MainActor in
            await MiddlePopup(state: .loading, onClose: cancelCheckUrlTask)
                .present()
        }
    }

    private func completeLoading(_ state: PopupState) {
        checkUrlTask = nil

        Task { @MainActor in
            await dismissAllPopups()

            let dismissAfter: Double = switch state {
            case .failure:
                7
            case .success:
                2
            default: 0
            }

            try? await Task.sleep(for: .seconds(1))
            await MiddlePopup(state: state)
                .dismissAfter(dismissAfter)
                .present()
        }
    }

    func checkAndSaveNode() {
        var node: Node? = nil

        do {
            node = try nodeSelector.parseCustomNode(url: customUrl, name: selectedNodeName, enteredName: customNodeName)
            customUrl = node?.url ?? customUrl
            customNodeName = node?.name ?? customNodeName
        } catch {
            showParseUrlAlert = true
            switch error {
            case let NodeSelectorError.ParseNodeUrlError(errorString):
                parseUrlMessage = errorString
            default:
                parseUrlMessage = "Unknown error \(error.localizedDescription)"
            }
        }

        if let node {
            Task {
                showLoadingPopup()
                let result = await Result { try await nodeSelector.checkAndSaveNode(node: node) }

                switch result {
                case .success:
                    refreshNodeState()
                    completeLoading(.success("Connected to node successfully"))
                case let .failure(error):
                    let errorMessage = "Failed to connect to node\n \(error.localizedDescription)"
                    let formattedMessage = errorMessage.replacingOccurrences(of: "\\n", with: "\n")

                    completeLoading(.failure(formattedMessage))
                }
            }
        }
    }

    private func nodeSelectionChanged(to newSelectedNodeName: String) {
        guard nodeSelector.selectedNode().name != newSelectedNodeName else { return }

        if newSelectedNodeName.hasPrefix("Custom") {
            restoreCustomNodeFields(for: newSelectedNodeName)
            return
        }

        guard let node = try? nodeSelector.selectPresetNode(name: newSelectedNodeName) else { return }

        showLoadingPopup()
        checkUrlTask = Task {
            do {
                try await nodeSelector.checkSelectedNode(node: node)
                refreshNodeState()
                completeLoading(.success("Succesfully connected to \(node.url)"))
            } catch {
                completeLoading(.failure("Failed to connect to \(node.url), reason: \(error.localizedDescription)"))
            }
        }
    }

    private func restoreCustomNodeFields(for selectedNodeName: String) {
        guard case let .custom(savedSelectedNode) = nodeSelector.selectedNode() else { return }

        let matchesApiType =
            savedSelectedNode.apiType == .electrum && selectedNodeName.contains("Electrum")
                || savedSelectedNode.apiType == .esplora && selectedNodeName.contains("Esplora")
        guard matchesApiType else { return }

        customUrl = savedSelectedNode.url
        customNodeName = savedSelectedNode.name
    }

    // MARK: Local node

    private func startPolling() {
        let interval = (isRunning && isInIbd == true) ? 0.5 : 2.0
        timer = Timer.scheduledTimer(withTimeInterval: interval, repeats: true) { _ in
            Task { await refreshLocalNodeState() }
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

    private func refreshLocalNodeState() async {
        do {
            let wasRunning = isRunning
            let wasIbd = isInIbd
            isRunning = await localNodeIsRunning()
            tipHeight = await localNodeTipHeight()
            isInIbd = await localNodeIsInIbd()
            peerCount = await localNodePeerCount()
            datadirSize = try await localNodeDatadirSize()
            isNetworkConnected = CloudConnectivityMonitor.shared.isConnected()

            if wasRunning != isRunning || wasIbd != isInIbd {
                await MainActor.run { restartPolling() }
            }

            let logs = await localNodeLogs(limit: 0)
            await MainActor.run {
                logLines = logs
            }
        } catch {
            datadirSize = nil
        }
    }

    private func startLocalNode() {
        Task {
            do {
                try await localNodeStart(network: app.selectedNetwork)
                await refreshLocalNodeState()
            } catch {
                await MainActor.run { errorMessage = error.localizedDescription }
            }
        }
    }

    private func stopLocalNode() {
        Task {
            await localNodeStop()
            await refreshLocalNodeState()
        }
    }

    private func clearLocalDatadir() {
        Task {
            do {
                try await localNodeClearDatadir()
                await refreshLocalNodeState()
            } catch {
                await MainActor.run { errorMessage = error.localizedDescription }
            }
        }
    }

    private func setLocalLogLevel(_ level: String) {
        logLevel = level
        Task {
            let ok = await localNodeSetLogLevel(level: level)
            if !ok {
                await MainActor.run { errorMessage = "Unknown log level: \(level)" }
            }
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

    // MARK: Body

    var body: some View {
        GeometryReader { geometry in
            ZStack {
                Form {
                    NodeSelectionPresetSection(
                        nodeList: nodeList,
                        selectedNodeName: $selectedNodeName
                    )
                    NodeSelectionCustomFields(
                        selectedNodeName: selectedNodeName,
                        customUrl: $customUrl,
                        customNodeName: $customNodeName,
                        save: checkAndSaveNode
                    )

                    Section("Local") {
                        LocalNodeStatusSection(
                            isRunning: isRunning,
                            tipHeight: tipHeight,
                            isInIbd: isInIbd,
                            datadirSize: datadirSize,
                            peerCount: peerCount,
                            horizon: horizonFromLogs,
                            isNetworkConnected: isNetworkConnected
                        )

                        LocalNodeActionsSection(
                            isRunning: isRunning,
                            onStart: startLocalNode,
                            onStop: stopLocalNode,
                            onClear: { showClearConfirm = true }
                        )

                        LocalNodeLogSection(
                            logLines: filteredLogLines,
                            logLevel: $logLevel,
                            onSetLogLevel: setLocalLogLevel
                        )
                        .frame(height: 240)
                    }
                }
                .scrollContentBackground(.hidden)

                if isMac && showPeersPanel {
                    HStack(spacing: 0) {
                        Spacer()

                        PeersPanelView(peerCount: peerCount, isRunning: isRunning)
                            .frame(width: geometry.size.width * peersPanelWidthRatio)
                            .background(Color(.systemBackground))
                            .overlay(
                                HStack(spacing: 0) {
                                    Rectangle()
                                        .fill(Color.gray.opacity(0.3))
                                        .frame(width: 4)
                                        .contentShape(Rectangle())
                                        .gesture(
                                            DragGesture()
                                                .onChanged { value in
                                                    let delta = -value.translation.width
                                                    let newWidth = (geometry.size.width * peersPanelWidthRatio) + delta
                                                    let clamped = min(max(newWidth, geometry.size.width * 0.25), geometry.size.width * 0.85)
                                                    peersPanelWidthRatio = clamped / geometry.size.width
                                                }
                                        )
                                    Spacer()
                                }
                            )
                            .shadow(radius: 4)
                    }
                    .transition(.move(edge: .trailing))
                    .animation(.easeInOut(duration: 0.25), value: showPeersPanel)
                }
            }
        }
        .navigationTitle("Node")
        .onAppear {
            Task {
                await localNodeSetLogLevel(level: "trace")
                await refreshLocalNodeState()
            }
            startPolling()
        }
        .onDisappear {
            stopPolling()
        }
        .onDisappear {
            // custom esplora or electrum is selected
            if showCustomUrlField { checkAndSaveNode() }
        }
        .onChange(of: selectedNodeName) { _, newSelectedNodeName in
            nodeSelectionChanged(to: newSelectedNodeName)
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
            Button("Clear", role: .destructive) { clearLocalDatadir() }
        } message: {
            Text("This will delete all local node data and require a full resync.")
        }
        .alert(isPresented: $showParseUrlAlert) {
            Alert(
                title: Text("Unable to parse URL"),
                message: Text(parseUrlMessage),
                dismissButton: .default(Text("OK")) {
                    showParseUrlAlert = false
                    parseUrlMessage = ""
                    Task { await dismissAllPopups() }
                }
            )
        }
        .onKeyPress(.init("\\")) {
            if isMac {
                showPeersPanel.toggle()
            }
            return .handled
        }
    }
}

// MARK: Node selection subviews

private struct NodeSelectionPresetSection: View {
    let nodeList: [NodeSelection]
    @Binding var selectedNodeName: String

    var body: some View {
        Section {
            ForEach(nodeList, id: \.name) { node in
                NodeSelectionRow(
                    name: node.name,
                    isSelected: selectedNodeName == node.name,
                    select: { selectedNodeName = node.name }
                )
            }
            NodeSelectionRow(
                name: "Custom Electrum",
                isSelected: selectedNodeName == "Custom Electrum",
                select: { selectedNodeName = "Custom Electrum" }
            )
            NodeSelectionRow(
                name: "Custom Esplora",
                isSelected: selectedNodeName == "Custom Esplora",
                select: { selectedNodeName = "Custom Esplora" }
            )
        }
    }
}

private struct NodeSelectionRow: View {
    let name: String
    let isSelected: Bool
    let select: () -> Void

    var body: some View {
        HStack {
            Text(name)
                .font(.subheadline)

            Spacer()

            if isSelected {
                Image(systemName: "checkmark")
                    .foregroundStyle(.blue)
                    .font(.footnote)
                    .fontWeight(.semibold)
            }
        }
        .contentShape(Rectangle())
        .onTapGesture(perform: select)
    }
}

private struct NodeSelectionCustomFields: View {
    let selectedNodeName: String
    @Binding var customUrl: String
    @Binding var customNodeName: String
    let save: () -> Void

    var body: some View {
        if selectedNodeName.hasPrefix("Custom") {
            Section(selectedNodeName) {
                NodeSelectionTextField(
                    title: "URL",
                    placeholder: "Enter URL",
                    text: $customUrl,
                    isUrl: true
                )
                NodeSelectionTextField(
                    title: "Name",
                    placeholder: "Node Name (optional)",
                    text: $customNodeName,
                    isUrl: false
                )
                Button("Save Custom Node", action: save)
                    .disabled(customUrl.isEmpty)
            }
        }
    }
}

private struct NodeSelectionTextField: View {
    let title: String
    let placeholder: String
    @Binding var text: String
    let isUrl: Bool

    var body: some View {
        HStack {
            Text(title)
                .frame(width: 60, alignment: .leading)

            TextField(placeholder, text: $text)
                .keyboardType(isUrl ? .URL : .default)
                .textInputAutocapitalization(.never)
        }
        .font(.subheadline)
    }
}

// MARK: Local node subviews

private struct LocalNodeStatusSection: View {
    let isRunning: Bool
    let tipHeight: UInt32?
    let isInIbd: Bool?
    let datadirSize: UInt64?
    let peerCount: UInt32?
    let horizon: UInt32?
    let isNetworkConnected: Bool

    var body: some View {
        VStack(alignment: .leading, spacing: 12) {
            Text("Status")
                .font(.subheadline)
                .fontWeight(.semibold)
                .foregroundStyle(.secondary)

            VStack(spacing: 0) {
                StatusRow(title: "State", value: isRunning ? "Running" : "Stopped")

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

private struct PeersPanelView: View {
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
    SettingsContainer(route: .node)
        .environment(AppManager.shared)
        .environment(AuthManager.shared)
}
