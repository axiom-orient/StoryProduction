# 12. Traceability Matrix

| Requirement | Contract/Owner | Verification |
|---|---|---|
| Narrative one-time interpretation | STS/NarrativePackage | no duplicate StoryIR + semantic preflight |
| multiple outputs | DeliveryRequest | output_id uniqueness + plan/set exact match |
| loss-controlled projection | DirectorPlan | lineage/content + FULL omission legality |
| semantic-bearing assets | AssetRequest | source provenance + brief semantic audit |
| shared identity assets | AssetIndex | same asset SHA reused across outputs |
| mixed providers | AssetIndex item provenance | multi-provider fixture |
| capability truth | MediaBackend DESCRIBE | typed descriptor + unsupported/ambiguous failure |
| timed determinism | DeliveryRequest/DirectorPlan | unit frame sum == requested frames |
| cancel/stale safety | Runtime | late owner cannot publish |
| semantic verification | preflight + DeliverySet | false actor/fact negative fixture |
| visual verification | DeliverySet | PASS/FAIL evidence coherence |
| technical verification | DeliverySet | file/hash/dimension/frame evidence |
| 5 durable contracts | contracts/ | exact schema set gate |

Definition of Done = **contract + reachable implementation + wiring + positive/negative regression evidence**.
