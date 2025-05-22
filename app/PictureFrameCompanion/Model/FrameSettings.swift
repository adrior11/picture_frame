//
//  FrameSettings.swift
//  PictureFrameCompanion
//
//  Created by Schneider, Adrian on 15.05.25.
//

import Foundation

/// Mirrors Rust `FrameSettings`
struct FrameSettings: Codable {
    var display_enabled: Bool
    var rotate_interval_secs: Int
    var shuffle: Bool
    var pinned_image: String?
}
