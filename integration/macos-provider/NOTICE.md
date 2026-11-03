# Notices

- `SEMIProviderSDK` is consumed from `../..` workspace sibling `SEMI-ControlPlane/Packages/SEMIProviderSDK` and is covered by [SEMI-ControlPlane's Apache-2.0 license](../../../SEMI-ControlPlane/LICENSE). Its protocol dependency notices remain in [the SDK notices](../../../SEMI-ControlPlane/Packages/SEMIProviderSDK/THIRD_PARTY_NOTICES.md).
- `swift-subprocess` is pinned to `1.0.0` in this package for bounded child-process I/O and cancellation teardown. It is Apache-2.0 with the Swift exception; see the [upstream source](https://github.com/swiftlang/swift-subprocess/tree/1.0.0) and [license](https://github.com/swiftlang/swift-subprocess/blob/1.0.0/LICENSE).
- `swift-system` is a transitive dependency of `swift-subprocess`; it is Apache-2.0 with the Swift exception. See the [upstream source](https://github.com/apple/swift-system) and [license](https://github.com/apple/swift-system/blob/main/LICENSE.txt).
