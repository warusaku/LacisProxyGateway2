# Omada OpenAPI Manual

LacisProxyGateway2 OmadaControl で使用する Omada OpenAPI エンドポイントの実機確認結果。

## 動作確認済みコントローラ

| 項目 | 値 |
|------|-----|
| Controller | OC200 (192.168.3.50) |
| controllerVer | 6.0.0.36 |
| apiVer | 3 |
| omadacId | (実機固有値) |

## 認証フロー

### 1. Controller Info (認証不要)

```
GET {base_url}/api/info
```

Response:
```json
{
  "errorCode": 0,
  "result": {
    "controllerVer": "6.0.0.36",
    "apiVer": "3",
    "configured": true,
    "omadacId": "xxxxxxxx"
  }
}
```

### 2. Token取得

```
POST {base_url}/openapi/authorize/token?grant_type=client_credentials
Content-Type: application/json

{
  "omadacId": "{omadac_id}",
  "client_id": "{client_id}",
  "client_secret": "{client_secret}"
}
```

Response:
```json
{
  "errorCode": 0,
  "result": {
    "accessToken": "...",
    "tokenType": "bearer",
    "expiresIn": 7200
  }
}
```

**注意**: `expiresIn` は秒単位。LPGでは安全マージン60秒を差し引いてキャッシュ。

### 認証ヘッダー

全APIリクエストに以下を付与:
```
Authorization: AccessToken={access_token}
```

## エンドポイント一覧

### Sites

```
GET /openapi/v1/{omadacId}/sites?page=1&pageSize=100
```

動作確認: OK (Akihabara_office 1サイト)

Response data:
```json
{
  "siteId": "...",
  "name": "Akihabara_office",
  "region": "...",
  "timeZone": "...",
  "scenario": "..."
}
```

### Devices

```
GET /openapi/v1/{omadacId}/sites/{siteId}/devices?page=1&pageSize=100
```

動作確認: OK (5台)

| デバイス | タイプ | モデル |
|---------|--------|--------|
| Gateway | gateway | ER707-M2 |
| Switch | switch | SG2210P |
| AP1 | ap | EAP653 |
| AP2 | ap | EAP653 |
| AP3 | ap | EAP653 |

Response data:
```json
{
  "mac": "XX-XX-XX-XX-XX-XX",
  "name": "...",
  "type": "gateway|switch|ap",
  "model": "ER707-M2",
  "ip": "192.168.3.1",
  "status": 1,
  "firmwareVersion": "..."
}
```

### Clients

```
GET /openapi/v1/{omadacId}/sites/{siteId}/clients?page=1&pageSize=100
```

動作確認: OK (31台、ページネーション対応)

Response data (主要フィールド):
```json
{
  "mac": "XX-XX-XX-XX-XX-XX",
  "name": "...",
  "hostName": "...",
  "ip": "192.168.3.xxx",
  "vendor": "Apple",
  "deviceType": "...",
  "connectType": 0,
  "wireless": true,
  "ssid": "network_name",
  "signalLevel": 3,
  "rssi": -45,
  "apMac": "...",
  "apName": "...",
  "trafficDown": 1234567890,
  "trafficUp": 987654321,
  "uptime": 3600,
  "lastSeen": 1707350400,
  "active": true
}
```

### WireGuard Peers

```
GET /openapi/v1/{omadacId}/sites/{siteId}/vpn/wireguard-peers?page=1&pageSize=100
```

動作確認: OK (11 peers, 2 interfaces)

Response data:
```json
{
  "id": "...",
  "name": "peer_name",
  "status": true,
  "interfaceId": "...",
  "interfaceName": "wg0",
  "publicKey": "...",
  "allowAddress": ["10.0.0.2/32"],
  "keepAlive": 25,
  "comment": "..."
}
```

**重要**: WireGuard Interface API (`/vpn/wireguard`) は 405 Method Not Allowed を返す。
Interface情報はPeerの `interfaceId`/`interfaceName` から取得する。

### 未サポートエンドポイント (ER707-M2)

| エンドポイント | ステータス | 備考 |
|--------------|-----------|------|
| Gateway WAN | 404 | ER707-M2では未サポート |
| Port Forwards | 404 | ER707-M2では未サポート |
| WireGuard Interface | 405 | API自体が存在しない |

## LPG データフロー

```
Omada Controller ──(OpenAPI)──> OmadaClient
                                    │
                            OmadaSyncer (60s)
                                    │
                                    ▼
                              MongoDB Collections
                            ┌───────────────────┐
                            │ omada_controllers  │ ← 登録情報+認証情報
                            │ omada_devices      │ ← gateway/switch/AP
                            │ omada_clients      │ ← 接続端末
                            │ omada_wg_peers     │ ← WireGuard Peers
                            └───────────────────┘
                                    │
                              REST API (13 endpoints)
                                    │
                                    ▼
                              Frontend (OmadaControl page)
```

## mobes2.0 互換マッピング

| Omada device_type | ProductType | NetworkDeviceType |
|-------------------|-------------|-------------------|
| gateway | 101 | Router |
| switch | 102 | Switch |
| ap | 103 | AccessPoint |
| (other) | 191 | Unknown |

- MAC正規化: 大文字12桁HEX（セパレータ除去）
- `lacis_id`: nullable（mobes2.0の`lacisIdService`が将来付与するまでnull）
