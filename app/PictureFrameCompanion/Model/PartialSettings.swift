//
//  PartialSettings.swift
//  PictureFrameCompanion
//
//  Created by Schneider, Adrian on 15.05.25.
//

import Foundation

/// For PATCH /api/settings
struct PartialSettings: Codable {
    var display_enabled: Bool?
    var rotate_interval_secs: Int?
    var shuffle: Bool?
}
