//
//  ConnectionView.swift
//  PictureFrameCompanion
//
//  Created by Schneider, Adrian on 13.05.25.
//

import SwiftData
import SwiftUI

struct ConnectionView: View {
    @Environment(\.dismiss) private var dismiss
    @Bindable var conn: Connection

    var body: some View {
        NavigationStack {
            Form {
                Section("Picture Frame URL") {
                    TextField("http://", text: $conn.baseURL)
                        .keyboardType(.URL)
                        .textInputAutocapitalization(.never)
                        .font(.system(.body, design: .monospaced))
                }
                Section("Auth Token") {
                    SecureField("Bearer token", text: $conn.bearerToken)
                        .textInputAutocapitalization(.never)
                        .font(.system(.body, design: .monospaced))
                }
            }
            .navigationTitle("Connection")
            .toolbar {
                ToolbarItem(placement: .confirmationAction) { Button("Done") { dismiss() } }
            }
        }
        .presentationDetents([.medium])
    }
}
