//
//  LocalNodeMainContent.swift
//  Cove
//

import SwiftUI

struct LocalNodeMainContent: View {
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
    let showPeersPanel: Bool
    @Binding var peersPanelWidthRatio: CGFloat
    let geometryWidth: CGFloat
    let isMac: Bool

    var body: some View {
        ZStack {
            VStack(spacing: 0) {
                LocalNodeStatusSection(
                    isRunning: isRunning,
                    endpointsReady: endpointsReady,
                    tipHeight: tipHeight,
                    isInIbd: isInIbd,
                    datadirSize: datadirSize,
                    peerCount: peerCount,
                    electrumUrl: electrumUrl,
                    esploraUrl: esploraUrl,
                    horizon: horizonFromLogs,
                    isNetworkConnected: isNetworkConnected
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

                if endpointsReady, electrumUrl != nil || esploraUrl != nil {
                    LocalNodeConnectionSection(
                        electrumUrl: electrumUrl,
                        esploraUrl: esploraUrl
                    )
                    .padding(.horizontal, 16)
                    .padding(.top, 16)
                }

                LocalNodeLogSection(
                    logLines: logLines,
                    logLevel: $logLevel,
                    onSetLogLevel: onSetLogLevel
                )
                .padding(.top, 16)
                .frame(maxHeight: .infinity)
            }

            if isMac, showPeersPanel {
                PeersPanelOverlay(
                    peerCount: peerCount,
                    isRunning: isRunning,
                    geometryWidth: geometryWidth,
                    peersPanelWidthRatio: $peersPanelWidthRatio,
                    showPeersPanel: showPeersPanel
                )
            }
        }
    }
}
