# CelestialGlobe フロントエンド再設計ドキュメント

## 1. 概要

### 1.1 目的
mobes2.0 TopologyManagerPort（211ファイル、推定25,000+行）のUI/UXをLPG2のOpenAPI/RESTバックエンドに適合させた形で忠実に再実装する。

### 1.2 前回の失敗原因
- mobes2.0の実ソースを読まずに架空の情報に基づいて実装した（大原則6「現場猫案件」違反）
- ノードサイズ、間隔、LOD、CSS、レイアウトすべてが実物と乖離

### 1.3 今回のアプローチ
- mobes2.0の全211ファイルを9エージェントで読了済み
- 現行LPG2のUI実装19ファイルを全確認済み
- Firebase → OpenAPI REST置換のマッピングを明示的に定義

---

## 2. アーキテクチャ比較

### 2.1 データフロー比較

| 項目 | mobes2.0 | LPG2 |
|------|----------|------|
| バックエンド | Firebase/Firestore + Cloud Functions | Rust axum + MongoDB |
| データ取得 | onSnapshot リアルタイム購読 | REST API (GET /api/topology/v2) |
| 書き込み | Callable Functions | REST API (PUT/POST/DELETE) |
| 認証 | Firebase Auth | LacisOath JWT / Local Auth |
| nodeOrder SSoT | Firestore `facilities/{fid}/nodeOrder/` | MongoDB `cg_node_order` |
| ノード構築 | `buildNodesFromNodeOrder()` | バックエンド側 `build_raw_topology()` |
| エッジ構築 | `buildEdgesFromNodes()` (フロントエンド) | バックエンド側 `build_raw_topology()` |
| ポーリング | 30秒間隔 (Phase 4) | フロントエンド fetch (手動/タイマー) |

### 2.2 コンポーネント構成比較

| mobes2.0 コンポーネント | 行数 | LPG2 現行 | LPG2 再実装方針 |
|-------------------------|------|-----------|-----------------|
| MindMapCanvas.tsx | 1107 | ✅ 存在（簡略版） | 完全再実装 |
| MindMapPage.tsx | 243 | page.tsx で代替 | page.tsx を拡張 |
| DeviceNodeWithLOD.tsx | 1062 | ✅ DeviceNode.tsx（簡略版） | 完全再実装 |
| InternetNode.tsx | - (DeviceNode内) | ✅ 存在 | 維持・改善 |
| ContextMenu.tsx | 534 | ❌ なし | 新規実装 |
| OutlineView.tsx | 298 | ✅ 存在（簡略版） | 完全再実装 |
| PropertyPanel.tsx | 863 | ✅ 存在（簡略版） | 完全再実装 |
| CanvasToolbar.tsx | 313 | Toolbar.tsx | 完全再実装 |
| Header.tsx | 255 | layout.tsx で代替 | LPG2はsidebar方式なので不要 |
| Tooltip.tsx | 81 | ✅ 存在 | 維持（zoom補正追加） |
| icons.tsx | 162 | ✅ 存在 | 維持・拡張 |
| SettingsModal.tsx | 919 | ❌ なし | Phase2以降 |
| NoteNode.tsx | 406 | ❌ なし | Phase2以降 |
| DragGuideOverlay.tsx | 135 | ❌ なし | Phase1で実装 |
| GhostNode.tsx | 278 | ❌ なし | Phase2以降（検出デバイス） |
| LogsModal.tsx | 437 | 別ページ /logs で実装済み | 不要 |
| CSVImporter/Exporter | 745+319 | ❌ なし | Phase2以降 |
| VpnPanel.tsx | 306 | 別ページ /wireguard | 不要 |
| TopologyEdge | - | ✅ TopologyEdge.tsx | 維持・改善 |
| Legend.tsx | - | ✅ 存在 | 維持 |
| ViewModeSelector | - | ✅ 存在 | 維持 |
| LogicDeviceDialog | - | ✅ 存在 | 維持 |

### 2.3 Store 構成比較

| mobes2.0 | 行数 | LPG2 現行 | LPG2 方針 |
|----------|------|-----------|-----------|
| useTopologyStore.ts | ~500 | ✅ 161行（簡略版） | 拡張 |
| useUIStateStore.ts | ~400 | ❌ なし | 新規（選択/ドラッグ/コンテキストメニュー） |
| useHistoryStore.ts | 173 | ❌ なし | Phase2以降（undo/redo） |
| useFeatureFlagStore.ts | 108 | ❌ なし | Phase2以降 |
| useSiteStore.ts | ~200 | ❌ なし | 不要（LPG2はシングルサイト） |
| topologyStore/ (18ファイル) | ~5000 | ❌ なし | Phase1で主要部分 |

### 2.4 Hooks 構成比較

| mobes2.0 hooks | LPG2 対応 | LPG2 方針 |
|----------------|-----------|-----------|
| canvas/useDragHandlers (362) | ❌ | Phase1で実装（reparent） |
| canvas/useDragReparent (358) | ❌ | Phase1で実装 |
| canvas/useEdgeProcessing (228) | ❌ | Phase1で実装 |
| canvas/useNodeActions (392) | ❌ | Phase1で実装 |
| canvas/useViewportController (266) | ❌ | Phase1で実装 |
| canvas/useSiblingGapPreview (277) | ❌ | Phase2以降 |
| useKeyboardShortcuts (258) | ❌ | Phase2以降 |
| useDetectedDevices (270) | ❌ | Phase2以降 |

### 2.5 CSS/スタイル比較

| mobes2.0 | 行数 | LPG2 現行 | LPG2 方針 |
|----------|------|-----------|-----------|
| topologyManager.css | 1942 | styles.css (163行) | 完全再実装 |
| tailwind.config.js | 98 | ✅ 存在（拡張済み） | 維持・微調整 |

**mobes2.0のCSSの主要特徴**:
- スコープドダークモード: `#topology-manager-root.topology-dark`
- 4段階LOD: `[data-lod="low"]`, `[data-lod="mid"]`, `[data-lod="high"]`, `[data-lod="full"]`
- ガラスモーフィズム: `.glass-card`, `.glass-card-strong`
- ノードアニメーション: spawn, vanish, move, update effects
- ドラッグ状態: `.is-dragging`, `.ghost-node`, `.drop-candidate`
- 兄弟ギャッププレビュー: `.sibling-gap-preview-line`
- dimmedノード: `.node-dimmed`（opacity: 0.35 + grayscale 60%）

---

## 3. Firebase → OpenAPI マッピング

### 3.1 データ取得

| mobes2.0 (Firebase) | LPG2 (REST API) |
|----------------------|------------------|
| `getNodeOrder(fid)` | `GET /api/topology/v2` |
| `buildNodesFromNodeOrder()` | バックエンド側で構築済み（TopologyV2Response.nodes） |
| `buildEdgesFromNodes()` | バックエンド側で構築済み（TopologyV2Response.edges） |
| `onSnapshot(nodeOrder)` 30秒ポーリング | `setInterval(fetchTopology, 30000)` |
| `getOmadaSiteSettings()` | LPG2では不要（Omadaは別ページ） |
| `getSiteNotificationSettings()` | LPG2では不要 |

### 3.2 書き込み操作

| mobes2.0 (Firebase Callable) | LPG2 (REST API) |
|-------------------------------|------------------|
| `celestialGlobe_updateDevice` | `PUT /api/topology/nodes/:id/label` |
| `celestialGlobe_reparentDevice` | `PUT /api/topology/nodes/:id/parent` |
| `celestialGlobe_updateTopology` (edge add/remove) | 現在なし → 追加が必要 |
| `deleteDevice()` → 非推奨 | `DELETE /api/topology/logic-devices/:id` |
| `toggleCollapse()` | `PUT /api/topology/nodes/:id/collapse` |
| `flipSubtree()` via nodeOrder | 新規API追加: `PUT /api/topology/nodes/:id/orientation` |
| `reorderSiblings()` via nodeOrder | 新規API追加: `PUT /api/topology/nodes/:id/order` |

### 3.3 新規追加が必要なバックエンドAPI

| エンドポイント | メソッド | 用途 |
|---------------|---------|------|
| `/api/topology/nodes/:id/order` | PUT | 兄弟間並び替え |
| `/api/topology/nodes/:id/orientation` | PUT | サブツリー左右反転 |

---

## 4. LPG2で省略するmobes2.0機能（根拠付き）

**省略根拠**: LPG2に該当するバックエンドデータソースが存在しない機能のみ省略。UIコンポーネントの簡略化は行わない（大原則5: 情報の等価性）。

| 機能 | 省略根拠 |
|------|----------|
| Firebase Auth (AuthContext) | LPG2は独自認証（LacisOath/Local Auth）で実装済み |
| Multi-facility (Mission7) | LPG2は単一インスタンス運用 |
| Site selector | LPG2は全デバイス統合表示 |
| aranea IoT panel | LPG2にaraneaDeviceバックエンドなし（別系統） |
| VLAN map editor | LPG2にVLANデータソースなし |
| SSID management | LPG2にSSIDデータソースなし |
| Notification destinations | LPG2に通知設定なし |
| Dynamic PropertyPanel (fieldSchema) | Phase0 feature flag（mobes2.0でもdisabled） |
| Detected devices (GhostNode) | Phase2（LPG2にdetected_devicesデータソースを追加時） |
| CSV import/export | Phase2 |
| NoteNode | Phase2 |
| Undo/Redo | Phase2 |
| Keyboard shortcuts | Phase2 |
| Bubble notifications | Phase2 |
| Alert panel | Phase2 |
| VPN panel | LPG2は /wireguard ページで実装済み |
| Logs modal | LPG2は /logs ページで実装済み |

---

## 5. Phase1 実装スコープ

### 5.1 保持するファイル（現行LPG2から）
- `types.ts` — 型定義（TopologyNodeV2, TopologyEdgeV2 等）
- `stores/useTopologyStore.ts` — Zustand store
- `lib/api.ts` 内の `topologyV2Api` セクション

### 5.2 完全再実装するファイル

#### 5.2.1 CSS/スタイル
- `styles.css` → mobes2.0 `topologyManager.css` からLPG2向けに移植
  - LOD 4段階（data-lod属性）
  - ガラスモーフィズム
  - ノードアニメーション（spawn/vanish/move）
  - ドラッグ状態
  - dimmedノード

#### 5.2.2 コアコンポーネント
- `components/MindMapCanvas.tsx` — ReactFlowキャンバス本体
  - DFSレイアウト（layoutTree）
  - ドラッグ&ドロップ（reparent対応）
  - コンテキストメニュー連携
  - LOD切替（bindLodSwitch）
  - MiniMap + Background
  - ズームスケジューラ

- `components/DeviceNode.tsx` — デバイスノード（LOD対応）
  - mobes2.0 DeviceNodeWithLOD.tsx (1062行) 準拠
  - p-3 コンパクトカード
  - ステータスドット (w-4 h-4 ring-2)
  - ステータスバッジ (MANUAL/STATIC等)
  - GWバッジ (gateway)
  - MAC表示
  - LacisID表示
  - ソースバッジ（Omada/OpenWrt/External/Manual）
  - 折りたたみドットリング
  - LogicDevice破線+teal
  - LOD 4段階（low=最小表示、mid=基本情報、high=詳細、full=全情報）

- `components/InternetNode.tsx` — インターネットノード
  - indigo gradient
  - Source handle (Right) のみ

- `components/TopologyEdge.tsx` — カスタムエッジ
  - wired/wireless/vpn/logical スタイル
  - LODに応じたラベル表示

#### 5.2.3 パネル・オーバーレイ
- `components/PropertyPanel.tsx` — プロパティパネル
  - mobes2.0 PropertyPanel.tsx (863行) + BasicSections (433行) 準拠
  - 基本情報セクション（名前、タイプ、ステータス、IP、MAC）
  - 識別子セクション（LacisID、source）
  - ノートセクション
  - Save/Revert フロー

- `components/ContextMenu.tsx` — 右クリックメニュー
  - ノードコンテキスト（子追加、兄弟追加、折りたたみ、削除）
  - ペインコンテキスト（新規デバイス追加）

- `components/OutlineView.tsx` — ツリービュー
  - 展開/折りたたみ
  - インライン編集
  - ステータスインジケーター
  - タイプバッジ

- `components/Toolbar.tsx` → `components/CanvasToolbar.tsx`
  - デバイスパレット（router, switch, ap, client, server, logic_device）
  - ドラッグ&ドロップでノード追加

- `components/DragGuideOverlay.tsx` — ドラッグガイド
  - reparent/free/reorder の操作ガイド表示

#### 5.2.4 インフラ
- `components/deviceNode/helpers.ts` — ステータス計算、バッジ定義
- `components/deviceNode/hooks.ts` — useZoom, useNodeTooltipContent
- `components/icons.tsx` — NetworkDeviceIcon（SVGレガシー + LPG2追加タイプ）
- `components/Tooltip.tsx` — zoom補正ツールチップ
- `constants.ts` — 色定義、レイアウト定数

#### 5.2.5 lib
- `lib/layoutTree.ts` — DFSツリーレイアウト（mobes2.0 layoutSimple.ts 準拠）
- `lib/lodSwitch.ts` — LODレベル切替（CSS data-lod属性）

#### 5.2.6 Store 拡張
- `stores/useTopologyStore.ts` — 拡張
  - ポーリング（30秒間隔）
  - reparent, reorder, flip のAPI呼び出し
- `stores/useUIStateStore.ts` — 新規
  - selectedNodeId/selectedNodeIds
  - contextMenu状態
  - drag状態
  - highlightedEdgeIds
  - isLayouting

### 5.3 削除するファイル
- `components/Legend.tsx` — mobes2.0に存在しない独自追加
- `components/LogicDeviceDialog.tsx` — ContextMenu経由に統合
- `components/ViewModeSelector.tsx` — page.tsx内に統合

---

## 6. ファイル構成（Phase1完了後）

```
celestial-globe/
├── page.tsx                           # ページエントリ（ViewMode切替含む）
├── types.ts                           # 型定義（現行維持）
├── constants.ts                       # 色・レイアウト定数
├── styles.css                         # 全CSS（LOD, glass, anim, drag）
├── stores/
│   ├── useTopologyStore.ts            # トポロジーデータ + API
│   └── useUIStateStore.ts             # UI状態（選択/ドラッグ/メニュー）
├── lib/
│   ├── layoutTree.ts                  # DFSツリーレイアウト
│   └── lodSwitch.ts                   # LODレベル切替
└── components/
    ├── MindMapCanvas.tsx              # ReactFlowメインキャンバス
    ├── DeviceNode.tsx                 # デバイスノード（LOD対応）
    ├── InternetNode.tsx               # インターネットノード
    ├── TopologyEdge.tsx               # カスタムエッジ
    ├── PropertyPanel.tsx              # プロパティパネル
    ├── ContextMenu.tsx                # 右クリックメニュー
    ├── OutlineView.tsx                # ツリービュー
    ├── CanvasToolbar.tsx              # フローティングツールバー
    ├── DragGuideOverlay.tsx           # ドラッグガイド
    ├── Tooltip.tsx                    # zoom補正ツールチップ
    ├── icons.tsx                      # デバイスアイコン
    └── deviceNode/
        ├── helpers.ts                 # ステータス計算、バッジ
        └── hooks.ts                   # useZoom, useNodeTooltipContent
```

---

## 7. mobes2.0 → LPG2 主要設計決定

### 7.1 LOD（Level of Detail）
mobes2.0と同一の4段階を採用:
- `low` (zoom < 0.40): ノード最小表示（ステータスドットのみ）
- `mid` (0.40 ≤ zoom < 0.90): 基本情報（ラベル + IP）
- `high` (0.90 ≤ zoom < 1.2): 詳細（MAC, LacisID, ソースバッジ）
- `full` (zoom ≥ 1.2): 全情報

CSS `data-lod` 属性で切替（React再レンダリング不要）。
ヒステリシス閾値で切替振動を防止。

### 7.2 ノードデザイン
mobes2.0 DeviceNodeWithLOD.tsx 準拠:
- Container: `mindmap-node relative rounded-lg shadow-lg p-3`
- Status dot: `w-4 h-4 rounded-full ring-2 ring-white dark:ring-dark-200`
- Badge area: `absolute -top-2 right-2 flex items-center gap-1`
- LogicDevice: 破線 `border-dashed border-2` + teal gradient

### 7.3 レイアウトエンジン
mobes2.0 layoutSimple.ts (705行) から移植:
- `layoutTree(rootId, nodes, edges, options)` → `{ nodes, depthMap }`
- Left/Right マインドマップ方向
- allDescendants / minimum スペーシングモード
- nodeOrder のparentMac + order でソート

### 7.4 ダークモード
mobes2.0のスコープドダークモード方式を採用:
- `#celestial-globe-root.cg-dark` クラスで制御
- LPG2は既に `<html class="dark">` で全体ダークモード
- CelestialGlobe内はTailwind `dark:` prefixを活用

### 7.5 ガラスモーフィズム
mobes2.0 `.glass-card` 準拠:
```css
.cg-glass-card {
  background: rgba(30, 30, 46, 0.75);
  backdrop-filter: blur(16px) saturate(150%);
  border: 1px solid rgba(255, 255, 255, 0.08);
  border-radius: 12px;
}
```

---

## 8. テスト計画

### 8.1 フロントエンド
```bash
npm run type-check   # TypeScript型チェック
npm run lint         # ESLint
npm run build        # ビルド成功確認
```

### 8.2 バックエンド（新規API追加時のみ）
```bash
cargo check          # 型チェック
cargo clippy         # Lint
cargo test           # テスト
```

### 8.3 Chrome実UIテスト
- [ ] ページロード: /celestial-globe が正常に表示
- [ ] ノード表示: 全ノードがコンパクトカード（p-3）で表示
- [ ] ステータスドット: online=緑, offline=灰, warning=黄
- [ ] ステータスバッジ: MANUAL/STATIC/各stateType表示
- [ ] GWバッジ: gateway型ノードにsky-500バッジ
- [ ] MAC表示: 🆔 形式
- [ ] LacisID表示: LacisID形式
- [ ] LODズーム: 4段階が切り替わる（low/mid/high/full）
- [ ] ノード間隔: 重なりなし
- [ ] エッジ: ノードと重ならない
- [ ] 折りたたみ: ドットリング表示、カウントバッジ
- [ ] LogicDevice: 破線+teal gradient
- [ ] reparent: ドラッグでノード移動→親変更
- [ ] ラベル編集: ダブルクリックで編集可能
- [ ] OutlineView: ツリー表示、選択同期
- [ ] PropertyPanel: 選択ノードの情報表示
- [ ] ContextMenu: 右クリックでメニュー表示
- [ ] CanvasToolbar: デバイスパレット表示
- [ ] ViewMode: mindmap/outline/split切替
- [ ] MiniMap: 表示・操作可能
- [ ] ガラスモーフィズム: パネル・ツールバーの透過表示

---

## 9. MECE確認

### カバレッジ
- mobes2.0の211ファイルすべてを分類・対応方針決定済み ✅
- LPG2の現行19ファイルすべてを確認済み ✅
- Phase1/Phase2の分類は「LPG2バックエンドのデータソース有無」で決定（恣意的省略なし）✅
- 省略する機能は全て根拠付き ✅

### 排他性
- 各ファイルの責務は単一責任原則に基づき分離 ✅
- Store (useTopologyStore/useUIStateStore) は状態の種類で分離 ✅
- コンポーネント間の依存は一方向（Store → Component → Helper） ✅

---

## 10. 実装順序（依存関係に基づく）

```
Phase1-Step0: 保持ファイル確認（types.ts, store, api.ts）
Phase1-Step1: styles.css 再実装（LOD, glass, anim, drag）
Phase1-Step2: constants.ts 更新
Phase1-Step3: stores/useUIStateStore.ts 新規
Phase1-Step4: lib/lodSwitch.ts 新規
Phase1-Step5: lib/layoutTree.ts 新規
Phase1-Step6: deviceNode/helpers.ts 再実装
Phase1-Step7: deviceNode/hooks.ts 再実装
Phase1-Step8: icons.tsx 再実装
Phase1-Step9: Tooltip.tsx 改善
Phase1-Step10: DeviceNode.tsx 完全再実装
Phase1-Step11: InternetNode.tsx 改善
Phase1-Step12: TopologyEdge.tsx 改善
Phase1-Step13: CanvasToolbar.tsx 完全再実装
Phase1-Step14: DragGuideOverlay.tsx 新規
Phase1-Step15: ContextMenu.tsx 新規
Phase1-Step16: PropertyPanel.tsx 完全再実装
Phase1-Step17: OutlineView.tsx 完全再実装
Phase1-Step18: MindMapCanvas.tsx 完全再実装
Phase1-Step19: stores/useTopologyStore.ts 拡張
Phase1-Step20: page.tsx 再実装
Phase1-Step21: ビルド・テスト・デプロイ
```
