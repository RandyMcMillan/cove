//
//  LocalNodeSettingsBody.swift
//  Cove
//

import SwiftUI

struct LocalNodeSettingsBody: View {
    let isRunning: Bool
    let endpointsReady: Bool
    let tipHeight: UInt32?
    let isInIbd: Bool?
    let datadirSize: UInt64?
    let peerCount: UInt32?
    let electrumUrl: String?
    let esploraUrl: String?
    let horizonFromLogs: UInt32?
    let isNetworkConnected: Bool
    @Binding var showClearConfirm: Bool
    let logLines: [String]
    @Binding var logLevel: String
    let onSetLogLevel: (String) -> Void
    let startNode: () -> Void
    let stopNode: () -> Void
    @Binding var showPeersPanel: Bool
    @Binding var peersPanelWidthRatio: CGFloat
    @Binding var errorMessage: String?
    let isMac: Bool
    let startPolling: () -> Void
    let stopPolling: () -> Void
    let clearDatadir: () -> Void
    let localNodeSetLogLevel: (String) async -> Bool
    let refreshStatus: () async -> Void
    let refreshLogs: () async -> Void

    @State private var showHelp = false

    var body: some View {
        LocalNodeSettingsContent(
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
            logLines: logLines,
            logLevel: $logLevel,
            onSetLogLevel: onSetLogLevel,
            startNode: startNode,
            stopNode: stopNode,
            showPeersPanel: $showPeersPanel,
            peersPanelWidthRatio: $peersPanelWidthRatio,
            isMac: isMac,
            startPolling: startPolling,
            stopPolling: stopPolling,
            localNodeSetLogLevel: localNodeSetLogLevel,
            refreshStatus: refreshStatus,
            refreshLogs: refreshLogs
        )
    }
}

private struct LocalNodeSettingsContent: View {
    let isRunning: Bool
    let endpointsReady: Bool
    let tipHeight: UInt32?
    let isInIbd: Bool?
    let datadirSize: UInt64?
    let peerCount: UInt32?
    let electrumUrl: String?
    let esploraUrl: String?
    let horizonFromLogs: UInt32?
    let isNetworkConnected: Bool
    @Binding var showClearConfirm: Bool
    let logLines: [String]
    @Binding var logLevel: String
    let onSetLogLevel: (String) -> Void
    let startNode: () -> Void
    let stopNode: () -> Void
    @Binding var showPeersPanel: Bool
    @Binding var peersPanelWidthRatio: CGFloat
    let isMac: Bool
    let startPolling: () -> Void
    let stopPolling: () -> Void
    let localNodeSetLogLevel: (String) async -> Bool
    let refreshStatus: () async -> Void
    let refreshLogs: () async -> Void

    var body: some View {
        GeometryReader { geometry in
            LocalNodeMainContent(
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
                logLines: logLines,
                logLevel: $logLevel,
                onSetLogLevel: onSetLogLevel,
                startNode: startNode,
                stopNode: stopNode,
                showPeersPanel: showPeersPanel,
                peersPanelWidthRatio: $peersPanelWidthRatio,
                geometryWidth: geometry.size.width,
                isMac: isMac
            )
        }
        .navigationTitle("Local Node")
        .toolbar {
            ToolbarItem(placement: .topBarTrailing) {
                LocalNodeHelpButton(electrumUrl: electrumUrl, esploraUrl: esploraUrl)
            }
        }
        .onAppear {
            Task {
                await localNodeSetLogLevel("trace")
                await refreshStatus()
                await refreshLogs()
            }
            startPolling()
        }
        .onDisappear {
            stopPolling()
        }
        .onKeyPress(.init("\\")) {
            if isMac {
                showPeersPanel.toggle()
            }
            return .handled
        }
    }
}

private struct LocalNodeHelpButton: View {
    let electrumUrl: String?
    let esploraUrl: String?

    @State private var showHelp = false

    var body: some View {
        Button {
            showHelp = true
        } label: {
            Image(systemName: "questionmark.circle")
        }
        .sheet(isPresented: $showHelp) {
            LocalNodeHelpView(electrumUrl: electrumUrl, esploraUrl: esploraUrl)
        }
    }
}
