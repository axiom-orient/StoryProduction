import Foundation

/// JSON at this adapter boundary preserves number tokens until an exact owner type is required.
/// Foundation's object decoder rounds some decimal/exponent tokens before CAS validation.
struct StoryJSONNumber {
  let token: String

  var unsignedInteger: UInt64? {
    guard !token.hasPrefix("-") else { return nil }
    let parts = token.lowercased().split(separator: "e", omittingEmptySubsequences: false)
    let mantissa = parts[0].split(separator: ".", omittingEmptySubsequences: false)
    let fractionCount = mantissa.count == 2 ? mantissa[1].count : 0
    var digits = String(mantissa.joined().drop(while: { $0 == "0" }))
    if digits.isEmpty { return 0 }
    guard let exponent = parts.count == 2 ? Int(parts[1]) : 0,
      exponent >= -digits.count, exponent <= 20 + fractionCount
    else { return nil }
    let scale = exponent - fractionCount
    if scale < 0 {
      let removeCount = -scale
      guard removeCount <= digits.count,
        digits.suffix(removeCount).allSatisfy({ $0 == "0" })
      else { return nil }
      digits.removeLast(removeCount)
    } else {
      guard digits.count + scale <= 20 else { return nil }
      digits += String(repeating: "0", count: scale)
    }
    return UInt64(digits)
  }
}

enum StoryJSON {
  static func object(_ text: String, maximumBytes: Int) throws -> [String: Any] {
    guard text.utf8.count <= maximumBytes else { throw StoryJSONError.tooLarge }
    var parser = Parser(bytes: Array(text.utf8))
    let value = try parser.value(depth: 0)
    parser.whitespace()
    guard parser.index == parser.bytes.count, let object = value as? [String: Any] else {
      throw StoryJSONError.invalid
    }
    return object
  }

  static func encode(_ object: [String: Any], maximumBytes: Int) throws -> String {
    var encoder = Encoder(maximumBytes: maximumBytes)
    try encoder.value(object, depth: 0)
    return String(decoding: encoder.bytes, as: UTF8.self)
  }

  private struct Parser {
    let bytes: [UInt8]
    var index = 0

    mutating func whitespace() {
      while index < bytes.count, [9, 10, 13, 32].contains(bytes[index]) { index += 1 }
    }

    mutating func value(depth: Int) throws -> Any {
      guard depth <= 64 else { throw StoryJSONError.tooDeep }
      whitespace()
      guard index < bytes.count else { throw StoryJSONError.invalid }
      switch bytes[index] {
      case 123:
        index += 1
        whitespace()
        var result: [String: Any] = [:]
        if take(125) { return result }
        while true {
          whitespace()
          let key = try string()
          guard result[key] == nil else { throw StoryJSONError.duplicateKey }
          whitespace()
          guard take(58) else { throw StoryJSONError.invalid }
          result[key] = try value(depth: depth + 1)
          whitespace()
          if take(125) { return result }
          guard take(44) else { throw StoryJSONError.invalid }
        }
      case 91:
        index += 1
        whitespace()
        var result: [Any] = []
        if take(93) { return result }
        while true {
          result.append(try value(depth: depth + 1))
          whitespace()
          if take(93) { return result }
          guard take(44) else { throw StoryJSONError.invalid }
        }
      case 34: return try string()
      case 116:
        try literal("true")
        return NSNumber(value: true)
      case 102:
        try literal("false")
        return NSNumber(value: false)
      case 110:
        try literal("null")
        return NSNull()
      case 45, 48...57: return try number()
      default: throw StoryJSONError.invalid
      }
    }

    mutating func take(_ byte: UInt8) -> Bool {
      guard index < bytes.count, bytes[index] == byte else { return false }
      index += 1
      return true
    }

    mutating func literal(_ text: String) throws {
      let value = Array(text.utf8)
      guard bytes.count - index >= value.count,
        Array(bytes[index..<(index + value.count)]) == value
      else { throw StoryJSONError.invalid }
      index += value.count
    }

    mutating func string() throws -> String {
      let start = index
      guard take(34) else { throw StoryJSONError.invalid }
      while index < bytes.count {
        let byte = bytes[index]
        index += 1
        if byte == 34 {
          guard
            let text = try JSONSerialization.jsonObject(
              with: Data(bytes[start..<index]), options: [.fragmentsAllowed]) as? String
          else { throw StoryJSONError.invalid }
          return text
        }
        guard byte >= 32 else { throw StoryJSONError.invalid }
        if byte == 92 {
          guard index < bytes.count else { throw StoryJSONError.invalid }
          index += 1
        }
      }
      throw StoryJSONError.invalid
    }

    mutating func number() throws -> StoryJSONNumber {
      let start = index
      _ = take(45)
      guard index < bytes.count else { throw StoryJSONError.invalid }
      if !take(48) {
        guard (49...57).contains(bytes[index]) else { throw StoryJSONError.invalid }
        while index < bytes.count, (48...57).contains(bytes[index]) { index += 1 }
      }
      if take(46) { try digits() }
      if take(101) || take(69) {
        if !take(43) { _ = take(45) }
        try digits()
      }
      return StoryJSONNumber(token: String(decoding: bytes[start..<index], as: UTF8.self))
    }

    mutating func digits() throws {
      let start = index
      while index < bytes.count, (48...57).contains(bytes[index]) { index += 1 }
      guard index > start else { throw StoryJSONError.invalid }
    }
  }

  private struct Encoder {
    let maximumBytes: Int
    var bytes = Data()

    mutating func append(_ data: Data) throws {
      guard data.count <= maximumBytes - bytes.count else { throw StoryJSONError.tooLarge }
      bytes.append(data)
    }

    mutating func text(_ text: String) throws { try append(Data(text.utf8)) }

    mutating func value(_ value: Any, depth: Int) throws {
      guard depth <= 64 else { throw StoryJSONError.tooDeep }
      switch value {
      case let number as StoryJSONNumber: try text(number.token)
      case let object as [String: Any]:
        try text("{")
        for (index, key) in object.keys.sorted().enumerated() {
          if index > 0 { try text(",") }
          try self.value(key, depth: depth + 1)
          try text(":")
          try self.value(object[key]!, depth: depth + 1)
        }
        try text("}")
      case let array as [Any]:
        try text("[")
        for (index, item) in array.enumerated() {
          if index > 0 { try text(",") }
          try self.value(item, depth: depth + 1)
        }
        try text("]")
      case is String, is NSNumber, is NSNull:
        try append(
          JSONSerialization.data(withJSONObject: value, options: [.fragmentsAllowed, .sortedKeys]))
      default: throw StoryJSONError.invalid
      }
    }
  }
}

enum StoryJSONError: Error, Equatable { case invalid, duplicateKey, tooLarge, tooDeep }
