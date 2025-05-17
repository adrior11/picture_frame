//
//  ContentView.swift
//  PictureFrameCompanion
//
//  Created by Schneider, Adrian on 13.05.25.
//

import PhotosUI
import SwiftData
import SwiftUI
import UniformTypeIdentifiers
import os

struct ContentView: View {
    @Environment(\.modelContext) private var ctx
    @Query private var conns: [Connection]

    @StateObject private var api: ApiClient
    @State private var pickerItem: PhotosPickerItem?
    @State private var activeSheet: SheetType?
    @State private var showError = false

    private let logger = Logger()

    // MARK: - Init

    init() {
        let saved =
            (try? ModelContext(.init(for: Connection.self))
                .fetch(FetchDescriptor<Connection>()).first) ?? Connection()

        let url =
            URL(string: saved.baseURL)
            ?? URL(string: "about:blank")!
        _api = StateObject(wrappedValue: ApiClient(baseURL: url) { saved.bearerToken })
    }

    // MARK: - Body

    var body: some View {
        NavigationStack {
            picturesList
                .toolbar {
                    toolbarLeadingItems
                    toolbarTrailingItems
                }
                .task {
                    guard api.isConfigured else {
                        return
                    }
                    await loadAll()
                }
                .onChange(of: pickerItem) {
                    Task {
                        await upload()
                    }
                }
        }
        .sheet(item: $activeSheet) { sheet in
            switch sheet {
            case .settings:
                SettingsView(settings: api.settings ?? defaultSettings()) { patch in
                    Task { await api.patchSettings(patch) }
                }
            case .connection:
                ConnectionView(conn: existingConn())
                    .onDisappear { applyConnectionChanges() }
            }
        }
        .alert(
            "Error", isPresented: $showError,
            actions: {
                Button("OK", role: .cancel) { api.error = nil }
            },
            message: {
                Text(api.error ?? "")
            }
        )
        .onChange(of: api.error) {
            showError = api.error != nil
        }
    }

    // MARK: - Subviews

    private var picturesList: some View {
        List(api.pictures) { pic in
            PictureRow(pic: pic) {
                Task { await api.delete(id: pic.id) }
            }
        }
        .overlay {
            if api.pictures.isEmpty {
                ContentUnavailableView("No Pictures", systemImage: "photo")
            }
        }
        .navigationTitle("Picture Frame")
        .refreshable {
            await loadAll()
        }
    }

    private var toolbarLeadingItems: some ToolbarContent {
        ToolbarItemGroup(placement: .navigationBarLeading) {
            Button {
                activeSheet = .settings
            } label: {
                Image(systemName: "gearshape")
            }
            .disabled(!api.reachable)
        }
    }

    private var toolbarTrailingItems: some ToolbarContent {
        ToolbarItemGroup(placement: .navigationBarTrailing) {
            PhotosPicker(
                selection: $pickerItem,
                matching: .images,
                photoLibrary: .shared()
            ) {
                Image(systemName: "plus")
            }
            .disabled(
                conns.first?.bearerToken.isEmpty ?? true || conns.first?.baseURL.isEmpty ?? true
                    || api.busy || !api.reachable)

            Button {
                activeSheet = .connection
            } label: {
                Image(systemName: "key.fill")
            }
        }
    }

    // MARK: - Actions

    private func loadAll() async {
        await api.fetchPictures()
        // Only try the 2nd call if the first one reached.
        guard api.reachable else { return }
        await api.fetchSettings()
    }
    
    private func upload() async {
        guard let item = pickerItem else { return }

        guard let data = try? await item.loadTransferable(type: Data.self) else {
            logger.warning("Could not load image data from picker item")
            return
        }

        let (fileExt, mime): (String, String) = {
            if item.supportedContentTypes.contains(.png) {
                return ("png", "image/png")
            } else {
                return ("jpg", "image/jpeg")
            }
        }()

        let filename = "\(UUID().uuidString).\(fileExt)"
        await api.upload(data: data, filename: filename, mime: mime)
        pickerItem = nil
    }

    private func applyConnectionChanges() {
        guard let c = conns.first,
            let newURL = URL(string: c.baseURL),
            !c.baseURL.isEmpty
        else { return }

        api.update(url: newURL) { c.bearerToken }
    }

    // MARK: - Helpers

    private func existingConn() -> Connection {
        if let c = conns.first { return c }
        let fresh = Connection()
        ctx.insert(fresh)
        return fresh
    }

    private func defaultSettings() -> FrameSettings {
        // fallback in case .settings is nil
        FrameSettings(
            display_enabled: true,
            rotate_enabled: true,
            rotate_interval_secs: 10,
            shuffle: false)
    }
}

// MARK: - Supporting types

private enum SheetType: Identifiable {
    case settings, connection
    var id: Int { hashValue }
}

private struct PictureRow: View {
    let pic: Picture
    let onDelete: () -> Void

    var body: some View {
        HStack {
            Text(pic.filename).lineLimit(1)
            Spacer()
            Text(
                Date(timeIntervalSince1970: TimeInterval(pic.added_at) / 1000),
                format: .dateTime.year().month().day()
            )
            .foregroundStyle(.secondary)
            .font(.footnote)
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
    ContentView()
        .modelContainer(for: Connection.self, inMemory: true)
}
