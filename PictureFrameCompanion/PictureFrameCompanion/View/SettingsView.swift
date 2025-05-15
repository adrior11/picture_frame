//
//  SettingsView.swift
//  PictureFrameCompanion
//
//  Created by Schneider, Adrian on 15.05.25.
//

import SwiftUI

struct SettingsView: View {
    @State var settings: FrameSettings
    var onSave: (PartialSettings) -> Void
    @Environment(\.dismiss) private var dismiss

    var body: some View {
        NavigationStack {
            Form {
                Toggle("Display Enabled", isOn: $settings.display_enabled)
                Toggle("Rotate Enabled", isOn: $settings.rotate_enabled)
                Stepper(
                    "Interval: \(settings.rotate_interval_secs)s",
                    value: $settings.rotate_interval_secs,
                    in: 1...3600)
                Toggle("Shuffle", isOn: $settings.shuffle)
            }
            .navigationTitle("Frame Settings")
            .toolbar {
                ToolbarItem(placement: .confirmationAction) {
                    Button("Save") {
                        let patch = PartialSettings(
                            display_enabled: settings.display_enabled,
                            rotate_enabled: settings.rotate_enabled,
                            rotate_interval_secs: settings.rotate_interval_secs,
                            shuffle: settings.shuffle
                        )
                        onSave(patch)
                        dismiss()
                    }
                }
            }
        }
        .presentationDetents([.medium])
    }
}
