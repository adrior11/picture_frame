//
//  Picture.swift
//  PictureFrameCompanion
//
//  Created by Schneider, Adrian on 13.05.25.
//

import Foundation

struct Picture: Identifiable, Decodable {
    let id: String
    let filename: String
    let added_at: Int64  // Unix millis
}
