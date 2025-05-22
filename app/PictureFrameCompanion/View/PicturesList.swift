//
//  PictureList.swift
//  PictureFrameCompanion
//
//  Created by Schneider, Adrian on 22.05.25.
//

import SwiftUI

struct PicturesList: View {
    let pictures: [Picture]
    let pinnedImage: String?
    let onTogglePin: (Picture) async -> Void
    let onDelete: (String) async -> Void
    let onRefresh: () async -> Void

    var body: some View {
        List(pictures) { pic in
            PictureRow(
                pic: pic,
                pinned: pic.filename == pinnedImage,
                onTogglePin: { Task { await onTogglePin(pic) } },
                onDelete: { Task { await onDelete(pic.id) } }
            )
        }
        .overlay {
            if pictures.isEmpty {
                ContentUnavailableView("No Pictures", systemImage: "photo")
            }
        }
        .navigationTitle("Picture Frame")
        .refreshable {
            await onRefresh()
        }
    }
}

#Preview {
    let pinned_file = UUID().uuidString

    NavigationStack {
        PicturesList(
            pictures: [
                Picture(
                    id: "test1",
                    filename: UUID().uuidString,
                    added_at: Int64(Int(Date().timeIntervalSince1970 * 1000))
                ),
                Picture(
                    id: "test2",
                    filename: pinned_file,
                    added_at: Int64(Int(Date().timeIntervalSince1970 * 1000))
                ),
            ],
            pinnedImage: pinned_file,
            onTogglePin: { _ in },
            onDelete: { _ in },
            onRefresh: {}
        )
    }
}
