//
//  Settings.swift
//  PictureFrameCompanion
//
//  Created by Schneider, Adrian on 13.05.25.
//

import SwiftData

@Model
final class Token {
    var bearerToken: String = ""

    init(token: String = "") { self.bearerToken = token }
}
