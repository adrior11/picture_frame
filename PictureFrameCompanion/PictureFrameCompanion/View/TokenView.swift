//
//  SettingsView.swift
//  PictureFrameCompanion
//
//  Created by Schneider, Adrian on 13.05.25.
//

import SwiftData
import SwiftUI

struct TokenView: View {
    @Environment(\.dismiss) private var dismiss
    @Bindable var token: Token

    var body: some View {
        NavigationStack {
            Form {
                SecureField("Token", text: $token.bearerToken)
                    .textInputAutocapitalization(.never)
                    .font(.system(.body, design: .monospaced))
            }
            .navigationTitle("Auth Token")
            .toolbar {
                ToolbarItem(placement: .confirmationAction) { Button("Done") { dismiss() } }
            }
        }
        .presentationDetents([.medium])
    }
}
