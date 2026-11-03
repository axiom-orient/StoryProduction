# 10. Artifact Identity, Digest, and Provenance

## 세 개념

| 개념 | 의미 | 위치 |
|---|---|---|
| semantic ID | 도메인 객체의 결정적 식별자 | body field when required |
| body digest | immutable body bytes의 무결성 | ArtifactRef.digest |
| commit ID | 저장·승인·검증 transaction의 식별자 | ArtifactCommit.commit_id |

## canonical bytes

모든 JSON artifact body는 RFC 8785 JCS로 canonicalize한다. digest는 `SHA-256(JCS(body))`이며 `sha256:<64 lowercase hex>`로 표기한다.

정밀 정수, 금액, timestamp 정밀도처럼 IEEE-754 변환으로 손실될 값은 JSON number가 아니라 규정된 decimal string 또는 구조체를 사용한다. NVC의 0..1 score처럼 계약상 number인 값은 finite JSON number만 허용한다.

## self-reference 금지

body는 자신의 ArtifactRef나 body digest를 포함하지 않는다. semantic ID가 필요하면 별도 identity projection으로 계산한다. commit metadata는 body 밖에 둔다.

## revision

같은 logical id의 body 변경은 revision 증가와 새 digest를 만든다. digest만 바뀌고 revision이 유지되는 commit은 거부한다.
