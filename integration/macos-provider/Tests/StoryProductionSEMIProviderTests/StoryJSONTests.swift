import Foundation
import Testing

@testable import StoryProductionSEMIProvider

@Test(arguments: [
  ("0", UInt64(0)), ("1.0", UInt64(1)), ("1e1", UInt64(10)),
  ("9007199254740993", UInt64(9_007_199_254_740_993)),
  ("9007199254740993e0", UInt64(9_007_199_254_740_993)),
  ("9.223372036854775807e18", UInt64(9_223_372_036_854_775_807)),
  ("18446744073709551615", UInt64.max),
  ("184467440737095516150e-1", UInt64.max),
])
func integerTokensPreserveTheFullOwnerUInt64Range(token: String, expected: UInt64) throws {
  let object = try StoryJSON.object("{\"revision\":\(token)}", maximumBytes: 65_536)
  let number = try #require(object["revision"] as? StoryJSONNumber)
  #expect(number.unsignedInteger == expected)
  #expect(try StoryJSON.encode(object, maximumBytes: 65_536) == "{\"revision\":\(token)}")
}

@Test(arguments: [
  "true", "false", "-1", "-0", "0.1", "9223372036854775806.5",
  "9007199254740993.1", "1.000000000000000000000000000000000000000001",
  "18446744073709551616", "18446744073709551617", "1e20", "1e9999999999999999999",
])
func nonUInt64CASInputsNeverWrapOrRoundIntoAnInteger(token: String) throws {
  let object = try StoryJSON.object("{\"revision\":\(token)}", maximumBytes: 65_536)
  #expect((object["revision"] as? StoryJSONNumber)?.unsignedInteger == nil)
}

@Test(arguments: [
  #"{"expected_revision":1,"expected_revision":0}"#,
  #"{"brief":{"tone":"first","tone":"last"}}"#,
  #"{"a":1,"\u0061":2}"#,
  #"{"blocks":[{"text":"first","text":"last"}]}"#,
])
func duplicateObjectKeysAreRejectedAtEveryDepth(json: String) {
  #expect(throws: StoryJSONError.duplicateKey) {
    try StoryJSON.object(json, maximumBytes: 65_536)
  }
}

@Test(arguments: [
  "[]", "{}{}", #"{"x":01}"#, #"{"x":1.}"#, #"{"x":1e}"#,
  #"{"x":+1}"#, #"{"x":NaN}"#, #"{"x":true,}"#, #"{"x":[1,]}"#,
  #"{"x":"\q"}"#,
])
func malformedJSONNeverProducesAnObject(json: String) {
  #expect(throws: (any Error).self) { try StoryJSON.object(json, maximumBytes: 65_536) }
}

@Test func strictCodecKeepsStringsArraysBooleansNullAndLimits() throws {
  let input = #"{"text":"한글 \" \\ \n","items":[true,false,null,1.25,{"v":"👩🏽‍🚀"}]}"#
  let object = try StoryJSON.object(input, maximumBytes: 65_536)
  let encoded = try StoryJSON.encode(object, maximumBytes: 65_536)
  #expect(
    try StoryJSON.object(encoded, maximumBytes: 65_536)["text"] as? String == object["text"]
      as? String)
  #expect(throws: StoryJSONError.tooLarge) { try StoryJSON.object(input, maximumBytes: 1) }
  #expect(throws: StoryJSONError.tooLarge) { try StoryJSON.encode(object, maximumBytes: 1) }
  let deep =
    "{\"x\":" + String(repeating: "[", count: 65) + "0" + String(repeating: "]", count: 65) + "}"
  #expect(throws: StoryJSONError.tooDeep) { try StoryJSON.object(deep, maximumBytes: 65_536) }
}
