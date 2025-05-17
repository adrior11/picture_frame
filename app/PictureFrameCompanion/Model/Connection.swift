//
//  Connection.swift
//  PictureFrameCompanion
//
//  Created by Schneider, Adrian on 16.05.25.
//

import SwiftData

@Model
final class Connection {
    var baseURL: String = ""
    var bearerToken: String = ""

    init(url: String = "", token: String = "") {
        self.baseURL = url
        self.bearerToken = token
    }
}
