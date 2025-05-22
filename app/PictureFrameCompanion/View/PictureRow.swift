//
//  PictureRow.swift
//  PictureFrameCompanion
//
//  Created by Schneider, Adrian on 22.05.25.
//

import SwiftUI

struct PictureRow: View {
    let pic: Picture
    let pinned: Bool
    let onTogglePin: () -> Void
    let onDelete: () -> Void

    var body: some View {
        HStack {
            if pinned {
                Image(systemName: "pin.fill")
                    .foregroundStyle(.blue)
            }

            Text(String(pic.filename.prefix(18))).lineLimit(1)

            Spacer()

            Text(
                Date(timeIntervalSince1970: TimeInterval(pic.added_at) / 1000),
                format: .dateTime.year().month().day()
            )
            .foregroundStyle(.secondary)
            .font(.footnote)
        }
        .contentShape(Rectangle())
        .onTapGesture {
            onTogglePin()
        }
        .swipeActions {
            Button(role: .destructive) {
                onDelete()
            } label: {
                Label("Delete", systemImage: "trash")
            }
        }
    }
}

#Preview {
    PictureRow(
        pic: Picture(
            id: "test",
            filename: UUID().uuidString,
            added_at: Int64(Int(Date().timeIntervalSince1970 * 1000))
        ),
        pinned: true,
        onTogglePin: {},
        onDelete: {}
    )
}
