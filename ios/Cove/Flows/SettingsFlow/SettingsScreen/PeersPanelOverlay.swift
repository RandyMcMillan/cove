//
//  PeersPanelOverlay.swift
//  Cove
//

import SwiftUI

struct PeersPanelOverlay: View {
    let peerCount: UInt32?
    let isRunning: Bool
    let geometryWidth: CGFloat
    @Binding var peersPanelWidthRatio: CGFloat
    let showPeersPanel: Bool

    var body: some View {
        HStack(spacing: 0) {
            Spacer()

            PeersPanelView(peerCount: peerCount, isRunning: isRunning)
                .frame(width: geometryWidth * peersPanelWidthRatio)
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
                                        let newWidth = (geometryWidth * peersPanelWidthRatio) + delta
                                        let clamped = min(max(newWidth, geometryWidth * 0.25), geometryWidth * 0.85)
                                        peersPanelWidthRatio = clamped / geometryWidth
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
