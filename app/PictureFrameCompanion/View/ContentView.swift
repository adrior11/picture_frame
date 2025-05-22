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
            PicturesList(
                pictures: api.pictures,
                pinnedImage: api.settings?.pinned_image,
                onTogglePin: togglePin,
                onDelete: api.delete,
                onRefresh: loadAll
            )
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

    // MARK: - Toolbar

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
        if api.reachable { await api.fetchSettings() }
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

    private func togglePin(_ pic: Picture) async {
        if api.settings?.pinned_image == pic.filename {
            await api.unpin(id: pic.id)
        } else {
            await api.pin(id: pic.id)
        }
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
            rotate_interval_secs: 10,
            shuffle: false,
            pinned_image: nil)
    }
}

// MARK: - Supporting types

private enum SheetType: Identifiable {
    case settings, connection
    var id: Int { hashValue }
}

#Preview {
    ContentView()
        .modelContainer(for: Connection.self, inMemory: true)
}
