//
//  ApiClient.swift
//  PictureFrameCompanion
//
//  Created by Schneider, Adrian on 13.05.25.
//

import Foundation
import SwiftUI
import os

@MainActor
final class ApiClient: ObservableObject {
    private let logger = Logger(subsystem: "PictureFrameCompanion", category: "api")
    private var token: () -> String

    @Published private(set) var reachable = false
    @Published var pictures: [Picture] = []
    @Published var settings: FrameSettings?
    @Published var busy = false
    @Published var error: String?
    @Published var baseURL: URL

    var isConfigured: Bool {
        !(baseURL.host ?? "").isEmpty && !token().isEmpty
    }

    // MARK: - Init & Update

    init(baseURL: URL, tokenProvider: @escaping () -> String) {
        self.baseURL = baseURL
        self.token = tokenProvider
    }

    func update(url: URL, tokenProvider: @escaping () -> String) {
        self.baseURL = url
        self.token = tokenProvider
    }

    // MARK: - Pictures

    /// GET /api/pictures
    func fetchPictures() async {
        await run {
            self.pictures = try await request(
                [Picture].self, method: "GET",
                path: "api/pictures")
        }
    }

    /// DELETE /api/pictures/{id}
    func delete(id: String) async {
        await run {
            _ = try await request(
                Empty.self, method: "DELETE",
                path: "api/pictures/\(id)")
            await fetchPictures()
        }
    }

    /// PUT  /api/pictures/{id}/pin
    func pin(id: String) async {
        await run {
            _ = try await request(
                Empty.self,
                method: "PUT",
                path: "api/pictures/\(id)/pin")
            await fetchSettings()
        }
    }

    /// DELETE /api/pictures/{id}/pin
    func unpin(id: String) async {
        await run {
            _ = try await request(
                Empty.self,
                method: "DELETE",
                path: "api/pictures/\(id)/pin")
            await fetchSettings()
        }
    }

    /// POST /api/pictures (multipart/form-data)
    func upload(data: Data, filename: String, mime: String) async {
        busy = true
        defer { busy = false }

        let boundary = "Boundary-\(UUID().uuidString)"

        var req = URLRequest(url: baseURL.appendingPathComponent("api/pictures"))
        req.httpMethod = "POST"
        req.setValue("Bearer \(token())", forHTTPHeaderField: "Authorization")
        req.setValue(
            "multipart/form-data; boundary=\(boundary)",
            forHTTPHeaderField: "Content-Type")

        // Multipart body
        var body = Data()
        body.append("--\(boundary)\r\n".data(using: .utf8)!)
        body.append(
            "Content-Disposition: form-data; name=\"file\"; filename=\"\(filename)\"\r\n".data(
                using: .utf8)!)
        body.append("Content-Type: \(mime)\r\n\r\n".data(using: .utf8)!)
        body.append(data)
        body.append("\r\n--\(boundary)--\r\n".data(using: .utf8)!)

        await run {
            let (respData, resp) = try await URLSession.shared.upload(
                for: req,
                from: body)
            guard (resp as? HTTPURLResponse)?.statusCode == 201 else {
                let text = String(data: respData, encoding: .utf8) ?? "<binary>"
                logger.error("Upload failed: \(text, privacy: .public)")
                throw URLError(.badServerResponse)
            }
            await fetchPictures()
        }
    }

    // MARK: - Frame Settings

    /// GET /api/settings
    func fetchSettings() async {
        await run {
            self.settings = try await request(
                FrameSettings.self,
                method: "GET",
                path: "api/settings")
        }
    }

    /// PATCH /api/settings
    func patchSettings(_ diff: PartialSettings) async {
        await run {
            let body = try JSONEncoder().encode(diff)
            self.settings = try await request(
                FrameSettings.self,
                method: "PATCH",
                path: "api/settings",
                headers: ["Content-Type": "application/json"],
                body: body)
        }
    }

    // MARK: - Private helpers

    private let session: URLSession = {
        let cfg = URLSessionConfiguration.ephemeral
        cfg.timeoutIntervalForRequest = 5
        cfg.timeoutIntervalForResource = 5
        return URLSession(configuration: cfg)
    }()

    private func request<T: Decodable>(
        _ type: T.Type = T.self,
        method: String,
        path: String,
        headers: [String: String] = [:],
        body: Data? = nil
    ) async throws -> T {
        var req = URLRequest(url: baseURL.appendingPathComponent(path))
        req.httpMethod = method
        req.setValue("Bearer \(token())", forHTTPHeaderField: "Authorization")
        headers.forEach { req.setValue($0.value, forHTTPHeaderField: $0.key) }
        req.httpBody = body

        //let (data, resp) = try await URLSession.shared.data(for: req)
        let (data, resp) = try await session.data(for: req)
        guard let http = resp as? HTTPURLResponse, http.statusCode < 300 else {
            throw URLError(.badServerResponse)
        }
        if T.self == Empty.self { return Empty() as! T }
        return try JSONDecoder().decode(T.self, from: data)
    }

    private func run(op: () async throws -> Void) async {
        do {
            try await op()
            reachable = true
        } catch {
            if let urlErr = error as? URLError {
                reachable = false
                self.pictures = []
                self.settings = nil
                self.error = message(for: urlErr)
                logger.error(
                    "URLError \(urlErr.code.rawValue): \(urlErr.localizedDescription, privacy: .public)"
                )
            } else {
                self.error = errorText(error)
                logger.error("API error: \(error.localizedDescription, privacy: .public)")
            }
        }
    }

    private func message(for err: URLError) -> String {
        switch err.code {
        case .timedOut, .cannotConnectToHost, .networkConnectionLost, .notConnectedToInternet:
            return "Cannot reach the picture frame right now."
        case .badServerResponse:
            return "The frame returned an unexpected response."
        default:
            return err.localizedDescription
        }
    }

    private func errorText(_ e: Error) -> String {
        (e as? LocalizedError)?.errorDescription ?? e.localizedDescription
    }

    private struct Empty: Decodable {}
}
