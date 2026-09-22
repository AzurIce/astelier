/* =====================================================================
 * Poke Image Studio — OpenAI Images 兼容协议调用工作台
 * 端点：POST {base}/images/generations（JSON，支持 SSE 流式）
 *       POST {base}/images/edits（本地文件 multipart / URL 列表 JSON）
 *       GET  {base}/models
 * ===================================================================== */
'use strict';

/* ---------- 常量 ---------- */
const LS_KEY = 'poke-image-studio-v1';
const GPT_IMAGE_MODELS = [
  'gpt-image-2', 'gpt-image-2.5-sunburst', 'gpt-image-2.5-flare',
];
// 旧版默认列表：localStorage 里检测到时自动迁移为精简列表（自定义列表不受影响）
const LEGACY_DEFAULT_MODELS = [
  'gpt-image-1', 'gpt-image-1-mini', 'gpt-image-1.5',
  'gpt-image-2', 'gpt-image-2-2026-04-21',
  'gpt-image-2.5-sunburst', 'gpt-image-2.5-sunburst-2026-09-08',
  'gpt-image-2.5-flare', 'gpt-image-2.5-flare-2026-09-08',
  'dall-e-2', 'dall-e-3',
];
const IMAGE_MODEL_PATTERN = /image|dall-?e|flux|banana|seedream|firefly|imagine|gen4|irag|hidream|qwen-image/i;

/* ---------- 模型能力元数据 ----------
 * 档案的核心是 params schema：表单参数区由它动态生成，
 * 能力树、请求体、校验同样由它驱动。接入新模型（如 banana）= 配一份元数据。
 *
 * param 描述符字段：
 *   key            发送到请求体的字段名
 *   label          表单显示名
 *   type           select | number | text | size（size 为复合控件：预设+自定义 WxH+规则提示）
 *   options        select/size 的可选值
 *   min/max        number 范围
 *   sizeCustom     size 是否允许自定义 WxH
 *   sizeRule       自定义分辨率的规则说明（展示用）
 *   modes          出现在哪些模式（缺省 = gen+edit 都显示）
 *   requiresStream true 时仅 stream 开启才发送
 */
function imgProfile(over) {
  return Object.assign({
    label: '',
    stream: true,
    maxPrompt: 32000,
    editsText: '未知，以网关为准',
    badges: [],
    params: [],
  }, over);
}

const PARAM_LIB = {
  quality: (opts) => ({ key: 'quality', label: 'quality 画质', type: 'select', options: opts }),
  size: (presets, sizeCustom, sizeRule) => ({ key: 'size', label: 'size 尺寸', type: 'size', options: presets, sizeCustom: !!sizeCustom, sizeRule: sizeRule || '' }),
  n: (max) => ({ key: 'n', label: 'n 数量', type: 'number', min: 1, max }),
  background: () => ({ key: 'background', label: 'background 背景', type: 'select', options: ['auto', 'transparent', 'opaque'] }),
  moderation: () => ({ key: 'moderation', label: 'moderation 审核', type: 'select', options: ['auto', 'low'] }),
  outputFormat: () => ({ key: 'output_format', label: 'output_format 输出格式', type: 'select', options: ['png', 'jpeg', 'webp'] }),
  outputCompression: () => ({ key: 'output_compression', label: 'output_compression 压缩率(0-100)', type: 'number', min: 0, max: 100 }),
  partialImages: () => ({ key: 'partial_images', label: 'partial_images 渐进帧(仅流式)', type: 'number', min: 0, max: 3, requiresStream: true }),
  user: () => ({ key: 'user', label: 'user 终端用户标识', type: 'text' }),
  style: () => ({ key: 'style', label: 'style 风格(dall-e-3)', type: 'select', options: ['vivid', 'natural'], modes: ['gen'] }),
  responseFormat: () => ({ key: 'response_format', label: 'response_format(dall-e)', type: 'select', options: ['url', 'b64_json'], modes: ['gen'] }),
  inputFidelity: () => ({ key: 'input_fidelity', label: 'input_fidelity 原图保真', type: 'select', options: ['low', 'high'], modes: ['edit'] }),
};

const SIZE_RULE_GPT2 = '边长需被 16 整除 · 宽高比 1:3–3:1 · 上限 3840×2160（超 2560×1440 为实验性）';

function gptCommonParams(qualityOpts, sizeCustom) {
  return [
    PARAM_LIB.quality(qualityOpts),
    PARAM_LIB.size(['auto', '1024x1024', '1536x1024', '1024x1536'], sizeCustom, SIZE_RULE_GPT2),
    PARAM_LIB.n(10),
    PARAM_LIB.background(),
    PARAM_LIB.moderation(),
    PARAM_LIB.outputFormat(),
    PARAM_LIB.outputCompression(),
    PARAM_LIB.partialImages(),
    PARAM_LIB.user(),
  ];
}

const MODEL_PROFILE_RULES = [
  { match: /^dall-e-2$/, profile: imgProfile({
    label: 'dall-e-2：quality 仅 standard；尺寸 256/512/1024 方图；不支持流式、背景、输出格式、moderation；response_format 可用；支持编辑端点与 mask。',
    badges: [['mask 编辑', 'ok'], ['256–1024 方图', 'info'], ['url/b64', 'info']],
    stream: false, maxPrompt: 1000,
    editsText: '支持编辑端点与 mask；input_fidelity 不支持',
    params: [
      PARAM_LIB.quality(['standard']),
      PARAM_LIB.size(['256x256', '512x512', '1024x1024'], false, ''),
      PARAM_LIB.n(10),
      PARAM_LIB.responseFormat(),
      PARAM_LIB.user(),
    ],
  }) },
  { match: /^dall-e-3$/, profile: imgProfile({
    label: 'dall-e-3：quality hd/standard；size 1024x1024 / 1792x1024 / 1024x1792；仅 n=1；独占 style（vivid/natural）与 response_format；不支持编辑端点。',
    badges: [['style', 'info'], ['url/b64', 'info'], ['不可编辑', 'off']],
    stream: false, maxPrompt: 4000,
    editsText: '不支持编辑端点',
    params: [
      PARAM_LIB.quality(['standard', 'hd']),
      PARAM_LIB.size(['1024x1024', '1792x1024', '1024x1792'], false, ''),
      PARAM_LIB.n(1),
      PARAM_LIB.style(),
      PARAM_LIB.responseFormat(),
      PARAM_LIB.user(),
    ],
  }) },
  { match: /^gpt-image-2\.5/, profile: imgProfile({
    label: 'gpt-image-2.5：画质独占 xhigh / max；透明背景完整支持；任意分辨率（边长被 16 整除、比例 1:3–3:1、上限 3840x2160）。',
    badges: [['任意尺寸', 'ok'], ['透明底', 'ok'], ['流式', 'ok'], ['xhigh/max', 'info'], ['mask 编辑', 'ok']],
    editsText: '支持：mask + 最多 16 张参考图 + input_fidelity（high/low）',
    params: gptCommonParams(['auto', 'high', 'medium', 'low', 'xhigh', 'max'], true),
  }) },
  { match: /^gpt-image-2/, profile: imgProfile({
    label: 'gpt-image-2：任意分辨率（边长被 16 整除、比例 1:3–3:1、上限 3840x2160，超 2560x1440 为实验性）；透明背景为预览特性。',
    badges: [['任意尺寸', 'ok'], ['透明底', 'ok'], ['流式', 'ok'], ['mask 编辑', 'ok']],
    editsText: '支持：mask + 最多 16 张参考图 + input_fidelity（high/low）；透明背景为预览特性',
    params: gptCommonParams(['auto', 'high', 'medium', 'low'], true),
  }) },
  { match: /^gpt-image/, profile: imgProfile({
    label: 'gpt-image-1 系：固定三档尺寸 + auto；quality high/medium/low；恒返 b64_json（response_format 无效）；支持 mask 编辑。',
    badges: [['透明底', 'ok'], ['流式', 'ok'], ['mask 编辑', 'ok']],
    editsText: '支持：mask + 最多 16 张参考图 + input_fidelity（high/low）',
    params: gptCommonParams(['auto', 'high', 'medium', 'low'], false),
  }) },
];

const GENERIC_PROFILE = imgProfile({
  label: '未识别的模型：按通用 OpenAI 兼容协议放行全部参数，实际支持以 Provider/网关为准；第三方模型经协议转换后 OpenAI 特有参数可能被忽略。',
  params: gptCommonParams(['auto', 'high', 'medium', 'low'], false),
});

function resolveProfile(modelId) {
  const id = String(modelId || '');
  let base = GENERIC_PROFILE;
  for (const r of MODEL_PROFILE_RULES) if (r.match.test(id)) { base = r.profile; break; }
  // agent 式覆盖：当前 Provider 对该模型的元数据配置优先于内置档案
  const ov = activeProvider()?.modelConfigs?.[id];
  return ov ? Object.assign({}, base, ov) : base;
}

const $ = (id) => document.getElementById(id);
const uid = () => Date.now().toString(36) + Math.random().toString(36).slice(2, 7);

/* ---------- 状态 ---------- */
const state = {
  providers: [],
  activeProvider: 0,
  editingProvider: -1,   // -1 = 关闭编辑器
  mode: 'gen',           // gen | edit
  pickedModel: { gen: null, edit: null }, // 每个模式各自记忆当前模型
  editFiles: [],         // [{ file, thumbUrl }]
  editMask: null,        // { blob, dataUrl, label }
  maskInvert: false,
  history: [],           // 内存态，不持久化
  lastImages: [],        // 最近一次结果（供「以最近结果」按钮使用）
};

function defaultProviders() {
  return [
    { name: 'OpenAI 官方', baseUrl: 'https://api.openai.com/v1', apiKey: '', proxy: '', models: [...GPT_IMAGE_MODELS], modelConfigs: {} },
    { name: 'Poke API', baseUrl: 'https://www.poke2api.com/v1', apiKey: '', proxy: '', models: [...GPT_IMAGE_MODELS], modelConfigs: {} },
  ];
}

function loadState() {
  try {
    const raw = localStorage.getItem(LS_KEY);
    if (raw) {
      const saved = JSON.parse(raw);
      if (Array.isArray(saved.providers) && saved.providers.length) {
        state.providers = saved.providers;
        state.activeProvider = Math.min(saved.activeProvider ?? 0, saved.providers.length - 1);
        for (const p of state.providers) {
          if (!p.modelConfigs || typeof p.modelConfigs !== 'object') p.modelConfigs = {};
          if (JSON.stringify(p.models) === JSON.stringify(LEGACY_DEFAULT_MODELS)) {
            p.models = [...GPT_IMAGE_MODELS];
          }
        }
      }
      if (saved.pickedModel && typeof saved.pickedModel === 'object') {
        state.pickedModel = {
          gen: saved.pickedModel.gen ?? null,
          edit: saved.pickedModel.edit ?? null,
        };
      }
    }
  } catch { /* 损坏的配置按默认处理 */ }
  if (!state.providers.length) state.providers = defaultProviders();
}

function saveState() {
  try {
    localStorage.setItem(LS_KEY, JSON.stringify({
      providers: state.providers,
      activeProvider: state.activeProvider,
      pickedModel: state.pickedModel,
    }));
  } catch { /* 忽略配额错误 */ }
}

const activeProvider = () => state.providers[state.activeProvider] ?? null;

/* ---------- 参数区（由模型元数据动态生成） ---------- */
function paramsFor(profile, mode) {
  return (profile.params || []).filter(d => !d.modes || d.modes.includes(mode));
}

function renderParamControl(prefix, d) {
  const id = `${prefix}P_${d.key}`;
  const optHTML = (opts) => opts.map(o => `<option value="${escapeHTML(o)}">${escapeHTML(o)}</option>`).join('');
  switch (d.type) {
    case 'select':
      return `<label>${escapeHTML(d.label)}
        <select id="${id}"><option value="">（不发送）</option>${optHTML(d.options || [])}</select>
      </label>`;
    case 'number':
      return `<label>${escapeHTML(d.label)}
        <input type="number" id="${id}" ${d.min != null ? `min="${d.min}"` : ''} ${d.max != null ? `max="${d.max}"` : ''} placeholder="留空不发送">
      </label>`;
    case 'text':
      return `<label>${escapeHTML(d.label)}
        <input type="text" id="${id}" placeholder="留空不发送" spellcheck="false">
      </label>`;
    case 'size': {
      const opts = [['', '（不发送）'], ...(d.options || []).map(o => [o, o])]
        .concat(d.sizeCustom ? [['custom', '自定义 WxH…']] : []);
      return `<label>${escapeHTML(d.label)}
        <select id="${id}">${opts.map(([v, t]) => `<option value="${escapeHTML(v)}">${escapeHTML(t)}</option>`).join('')}</select>
      </label>
      <div id="${id}_custom" class="size-custom-grid"${d.sizeCustom ? '' : ' hidden'}>
        <div class="row gap">
          <label class="grow">宽<input type="number" id="${id}_w" min="16" max="3840" step="16" value="1536"></label>
          <label class="grow">高<input type="number" id="${id}_h" min="16" max="2160" step="16" value="864"></label>
        </div>
        <div class="hint size-hint" id="${id}_hint"></div>
      </div>`;
    }
    default:
      return '';
  }
}

function renderParamsArea(prefix, mode) {
  const profile = resolveProfile(resolveModel(prefix));
  const defs = paramsFor(profile, mode);
  return `<div class="common-title">参数（由「${escapeHTML(resolveModel(prefix) || '当前模型')}」的元数据生成 · 选「不发送 / 留空」则省略）</div>`
    + defs.map(d => renderParamControl(prefix, d)).join('');
}

/* ---------- Provider 与模型侧边栏 ---------- */
function renderProviders() {
  const sel = $('providerQuickSelect');
  sel.innerHTML = state.providers
    .map((p, i) => `<option value="${i}" ${i === state.activeProvider ? 'selected' : ''}>${escapeHTML(p.name || '未命名')}</option>`)
    .join('');
}

function ensurePicks() {
  const p = activeProvider();
  const models = p?.models?.length ? p.models : GPT_IMAGE_MODELS;
  for (const mode of ['gen', 'edit']) {
    if (!state.pickedModel[mode] || !models.includes(state.pickedModel[mode])) {
      state.pickedModel[mode] = models[0];
    }
  }
}

function profileBadges(p, overridden) {
  // 内置档案用策展徽章；被元数据覆盖的模型从 params 推导，保证所见即所配
  if (!overridden && p.badges && p.badges.length) return p.badges;
  const params = p.params || [];
  const find = k => params.find(d => d.key === k);
  const b = [];
  if (find('size')?.sizeCustom) b.push(['任意尺寸', 'ok']);
  if (find('background')) b.push(['透明底', 'ok']);
  if (p.stream) b.push(['流式', 'ok']);
  const q = find('quality');
  if (q && (q.options.includes('xhigh') || q.options.includes('max'))) b.push(['xhigh/max', 'info']);
  if (find('style')) b.push(['style', 'info']);
  if (find('response_format')) b.push(['url/b64', 'info']);
  return b.length ? b : [['网关模型', 'warn']];
}

/* 能力树：全部由元数据推导 */
function profileTree(p) {
  const params = p.params || [];
  const find = k => params.find(d => d.key === k);
  const rows = [];
  const sz = find('size');
  rows.push(['尺寸', sz ? [
    ['预设', sz.options.join(' / ')],
    ['自定义', sz.sizeCustom ? `任意 WxH（${sz.sizeRule || '规则见文档'}）` : '不支持，仅预设档'],
  ] : [['元数据', '未定义 size 参数']]]);
  const q = find('quality');
  if (q) rows.push(['画质', q.options.join(' / ')]);
  const nD = find('n');
  rows.push(['数量', nD ? (nD.max === 1 ? '仅 n = 1' : `n ≤ ${nD.max}`) : 'n = 1（默认）']);
  const has = k => params.some(d => d.key === k);
  rows.push(['背景', has('background') ? 'transparent / opaque / auto' : '不支持']);
  rows.push(['流式', p.stream ? '支持；partial_images 0–3 帧渐进' : '不支持']);
  rows.push(['输出', has('output_format')
    ? `png / jpeg / webp${has('output_compression') ? '（压缩率 0–100）' : ''}`
    : (find('response_format') ? 'url / b64_json' : '固定返回格式')]);
  if (has('moderation')) rows.push(['审核', 'auto / low']);
  if (has('input_fidelity')) rows.push(['原图保真', 'low / high']);
  if (has('style')) rows.push(['style', 'vivid / natural']);
  if (has('response_format')) rows.push(['response_format', 'url / b64_json']);
  for (const d of params) {
    if (!['size', 'quality', 'n', 'background', 'moderation', 'output_format', 'output_compression', 'input_fidelity', 'style', 'response_format', 'user', 'partial_images'].includes(d.key)) {
      rows.push([d.label || d.key, d.type === 'select' ? (d.options || []).join(' / ') : d.type]);
    }
  }
  rows.push(['编辑', p.editsText]);
  rows.push(['Prompt 上限', `${p.maxPrompt} 字符`]);
  return rows;
}

const collapsedModels = new Set(); // 能力树默认全展开，这里记录被手动收起的

function renderModelList() {
  const p = activeProvider();
  const models = p?.models?.length ? p.models : GPT_IMAGE_MODELS;
  const list = $('modelList');
  list.innerHTML = models.map(m => {
    const prof = resolveProfile(m);
    const active = state.pickedModel[state.mode] === m ? ' active' : '';
    const open = !collapsedModels.has(m);
    const overridden = !!activeProvider()?.modelConfigs?.[m];
    const badges = profileBadges(prof, overridden)
      .map(([t, k]) => `<span class="badge ${k}">${escapeHTML(t)}</span>`)
      .join('');
    const tree = profileTree(prof).map(([label, leaves]) => {
      const items = Array.isArray(leaves)
        ? leaves.map(([k, v]) => `<div class="t-leaf"><span class="t-k">${escapeHTML(k)}</span><span class="t-v">${escapeHTML(v)}</span></div>`).join('')
        : `<div class="t-leaf"><span class="t-v only">${escapeHTML(leaves)}</span></div>`;
      return `<div class="t-node"><div class="t-label">${escapeHTML(label)}</div>${items}</div>`;
    }).join('');
    return `<div class="model-item${active}" data-model="${escapeHTML(m)}">
      <div class="m-row">
        <button class="m-toggle" data-toggle="${escapeHTML(m)}" title="展开 / 收起能力详情">${open ? '▾' : '▸'}</button>
        <span class="m-name">${escapeHTML(m)}</span>
        <button class="m-cfg" data-cfg="${escapeHTML(m)}" title="编辑该模型的元数据配置">⚙</button>
      </div>
      <div class="m-badges">${badges}</div>
      <div class="m-tree"${open ? '' : ' hidden'}>${tree}</div>
    </div>`;
  }).join('');
}

function pickModel(model) {
  if (!model) return;
  state.pickedModel[state.mode] = model;
  saveState();
  renderModelList();
  applyModelCapabilities(state.mode);
}

function refreshModelSidebar() {
  ensurePicks();
  renderModelList();
  applyModelCapabilities('gen');
  applyModelCapabilities('edit');
}

function resolveModel(prefix) {
  return state.pickedModel[prefix] || '';
}

function openProviderEditor(i) {
  state.editingProvider = i;
  const p = state.providers[i];
  $('providerEditorTitle').textContent = `编辑 Provider · ${p.name || '未命名'}`;
  $('pName').value = p.name ?? '';
  $('pBaseUrl').value = p.baseUrl ?? '';
  $('pApiKey').value = p.apiKey ?? '';
  $('pProxy').value = p.proxy ?? '';
  $('pModels').value = (p.models ?? []).join('\n');
  $('providerEditor').hidden = false;
}

function closeProviderEditor() {
  state.editingProvider = -1;
  $('providerEditor').hidden = true;
}

function saveProviderFromEditor() {
  const i = state.editingProvider;
  if (i < 0) return;
  const p = state.providers[i];
  p.name = $('pName').value.trim() || '未命名';
  p.baseUrl = normalizeBase($('pBaseUrl').value.trim());
  p.apiKey = $('pApiKey').value.trim();
  p.proxy = $('pProxy').value.trim();
  p.models = $('pModels').value.split('\n').map(s => s.trim()).filter(Boolean);
  saveState();
  renderProviders();
  refreshModelSidebar();
  closeProviderEditor();
}

function normalizeBase(url) {
  if (!url) return '';
  return url.replace(/\/+$/, '');
}

async function fetchModels() {
  const p = activeProvider();
  const base = normalizeBase(p.baseUrl || '');
  if (!base) { status('请先在「管理」中填写 Base URL', 'error'); return; }
  status('<span class="spin">◌</span> 拉取模型列表中…', 'info');
  try {
    const resp = await fetch(base + '/models', {
      headers: p.apiKey ? { Authorization: `Bearer ${p.apiKey}` } : {},
    });
    const json = await resp.json().catch(() => ({}));
    if (!resp.ok) throw new Error(json?.error?.message || `HTTP ${resp.status}`);
    const ids = (json.data ?? []).map(m => m.id).filter(Boolean);
    const imageOnes = ids.filter(id => IMAGE_MODEL_PATTERN.test(id));
    p.models = imageOnes.length ? imageOnes : ids;
    saveState();
    refreshModelSidebar();
    status(`✅ 共 ${ids.length} 个模型，已填入 ${p.models.length} 个${imageOnes.length ? '（已按图像模型过滤）' : ''}`, 'ok');
  } catch (err) {
    status('❌ 拉取失败：' + err.message, 'error');
  }
}

/* ---------- 能力应用与参数收集（元数据驱动） ---------- */
function modelHint(name) {
  return resolveProfile(name).label;
}

/* 按当前模型的元数据重建参数区 */
function applyModelCapabilities(prefix) {
  const model = resolveModel(prefix);
  const profile = resolveProfile(model);
  const mode = prefix === 'gen' ? 'gen' : 'edit';

  $(`${prefix}Common`).innerHTML = renderParamsArea(prefix, mode);

  // size 复合控件的联动
  const sizeSel = $(`${prefix}P_size`);
  if (sizeSel) {
    sizeSel.addEventListener('change', () => {
      $(`${prefix}P_size_custom`).hidden = sizeSel.value !== 'custom';
      updateSizeHint(prefix);
    });
    for (const s of ['_w', '_h']) {
      $(`${prefix}P_size${s}`).addEventListener('input', () => updateSizeHint(prefix));
    }
  }

  const streamCb = $(`${prefix}Stream`);
  streamCb.disabled = !profile.stream;
  streamCb.closest('label').classList.toggle('disabled', !profile.stream);
  if (!profile.stream) streamCb.checked = false;

  const ta = $(`${prefix}Prompt`);
  ta.maxLength = profile.maxPrompt;
  $(`${prefix}PromptCount`).textContent = `${ta.value.length} / ${profile.maxPrompt}`;

  const chip = $(`${prefix}CurrentModel`);
  if (chip) chip.textContent = model || '—';
  $(`${prefix}ModelHint`).textContent = modelHint(model);
  updateSizeHint(prefix);
}

/* 自定义尺寸约束：显示规则 + 实时校验 */
function updateSizeHint(prefix) {
  const profile = resolveProfile(resolveModel(prefix));
  const d = paramsFor(profile, prefix === 'gen' ? 'gen' : 'edit').find(x => x.key === 'size');
  const hintEl = $(`${prefix}P_size_hint`);
  if (!d || !hintEl) return;
  if (!d.sizeCustom || $(`${prefix}P_size`).value !== 'custom') {
    hintEl.textContent = d.sizeCustom ? (d.sizeRule || '') : '';
    hintEl.classList.remove('err', 'ok');
    return;
  }
  const w = Number($(`${prefix}P_size_w`).value), h = Number($(`${prefix}P_size_h`).value);
  const err = validateCustomSize(w, h);
  hintEl.textContent = err ? `✗ ${err}｜规则：${d.sizeRule || ''}` : `✓ ${w}×${h} 可用`;
  hintEl.classList.toggle('err', !!err);
  hintEl.classList.toggle('ok', !err);
}

function updatePromptCounter(prefix) {
  const ta = $(`${prefix}Prompt`);
  $(`${prefix}PromptCount`).textContent = `${ta.value.length} / ${ta.maxLength}`;
}

/* 遍历元数据 params，从动态控件收集请求体 */
function collectCommonParams(prefix, { forStream, mode }) {
  const profile = resolveProfile(resolveModel(prefix));
  const body = {};
  for (const d of paramsFor(profile, mode || (prefix === 'gen' ? 'gen' : 'edit'))) {
    const el = $(`${prefix}P_${d.key}`);
    if (!el) continue;
    if (d.type === 'size') {
      if (el.value === 'custom') {
        const w = Number($(`${prefix}P_size_w`).value), h = Number($(`${prefix}P_size_h`).value);
        const err = validateCustomSize(w, h);
        if (err) throw new Error(err);
        body.size = `${w}x${h}`;
      } else if (el.value) {
        body.size = el.value;
      }
    } else if (d.type === 'number') {
      if (el.value === '') continue;
      if (d.requiresStream && !forStream) continue;
      body[d.key] = Number(el.value);
    } else if (el.value !== '') {
      body[d.key] = el.value.trim();
    }
  }
  if (forStream) body.stream = true;
  return body;
}

function validateCustomSize(w, h) {
  if (!Number.isInteger(w) || !Number.isInteger(h) || w < 16 || h < 16) return '自定义尺寸必须是 ≥16 的整数';
  if (w % 16 !== 0 || h % 16 !== 0) return '自定义尺寸的宽和高都必须能被 16 整除';
  const ratio = w / h;
  if (ratio < 1 / 3 || ratio > 3) return '宽高比必须在 1:3 到 3:1 之间';
  if (w > 3840 || h > 2160) return '分辨率上限 3840x2160';
  return null;
}

function validateCustomSize(w, h) {
  if (!Number.isInteger(w) || !Number.isInteger(h) || w < 16 || h < 16) return '自定义尺寸必须是 ≥16 的整数';
  if (w % 16 !== 0 || h % 16 !== 0) return '自定义尺寸的宽和高都必须能被 16 整除';
  const ratio = w / h;
  if (ratio < 1 / 3 || ratio > 3) return '宽高比必须在 1:3 到 3:1 之间';
  if (w > 3840 || h > 2160) return '分辨率上限 3840x2160';
  return null;
}

/* ---------- 请求构造 ---------- */
function providerTarget() {
  const p = activeProvider();
  if (!p) throw new Error('没有可用的 Provider');
  const base = normalizeBase(p.baseUrl || '');
  if (!base) throw new Error('请先在左侧 Provider 中填写 Base URL');
  const url = (path) => p.proxy ? p.proxy.replace(/\/?$/, '') + base + path : base + path;
  const headers = {};
  if (p.apiKey) headers.Authorization = `Bearer ${p.apiKey}`;
  return { p, url, headers };
}

function buildGenRequest() {
  const model = resolveModel('gen');
  const prompt = $('genPrompt').value;
  if (!model) throw new Error('请选择或填写模型');
  if (!prompt.trim()) throw new Error('请填写 prompt');
  const { url, headers } = providerTarget();
  const body = { model, prompt, ...collectCommonParams('gen', { forStream: $('genStream').checked, mode: 'gen' }) };
  const endpoint = url('/images/generations');
  return {
    endpoint, headers,
    init: { method: 'POST', headers: { ...headers, 'Content-Type': 'application/json' }, body: JSON.stringify(body) },
    body, multipart: null,
    describe: curlForJSON(endpoint, body),
  };
}

function buildEditRequest() {
  const model = resolveModel('edit');
  const prompt = $('editPrompt').value;
  if (!model) throw new Error('请选择或填写模型');
  if (!prompt.trim()) throw new Error('请填写编辑指令 prompt');
  const { p, url, headers } = providerTarget();
  const files = state.editFiles.map(x => x.file);
  const urlLines = $('editImageUrls').value.split('\n').map(s => s.trim()).filter(Boolean);

  if (files.length && urlLines.length) {
    throw new Error('本地文件与 URL 参考图只能二选一：请清空其中一种');
  }
  if (!files.length && !urlLines.length) {
    throw new Error('请上传至少 1 张参考图，或填写参考图 URL');
  }

  const streamOn = $('editStream').checked;
  const common = collectCommonParams('edit', { forStream: streamOn, mode: 'edit' });

  if (files.length) {
    const fd = new FormData();
    fd.append('model', model);
    fd.append('prompt', prompt);
    const field = files.length > 1 ? 'image[]' : 'image';
    files.forEach(f => fd.append(field, f, f.name));
    if (state.editMask?.blob) fd.append('mask', state.editMask.blob, 'mask.png');
    for (const [k, v] of Object.entries(common)) {
      if (k === 'stream' && v === true) continue; // 流式走 init.stream 标记
      fd.append(k, typeof v === 'object' ? JSON.stringify(v) : String(v));
    }
    if (streamOn) fd.append('stream', 'true');
    const endpoint = url('/images/edits');
    return {
      endpoint, headers,
      init: { method: 'POST', headers, body: fd },
      streamFlag: streamOn,
      multipart: fd,
      body: null,
      describe: curlForMultipart(endpoint, model, prompt, files, !!state.editMask?.blob, common, streamOn),
    };
  }

  // JSON 模式：URL / file_id 列表
  const images = urlLines.map(line =>
    line.startsWith('fileid:') ? { file_id: line.slice(7).trim() } : { image_url: line }
  );
  const body = { model, prompt, images, ...common };
  const maskRef = $('editMaskUrl').value.trim();
  if (maskRef) body.mask = maskRef.startsWith('fileid:') ? { file_id: maskRef.slice(7).trim() } : { image_url: maskRef };
  const endpoint = url('/images/edits');
  return {
    endpoint, headers,
    init: { method: 'POST', headers: { ...headers, 'Content-Type': 'application/json' }, body: JSON.stringify(body) },
    streamFlag: streamOn,
    multipart: null, body,
    describe: curlForJSON(endpoint, body),
  };
}

/* ---------- cURL 预览 ---------- */
function curlForJSON(endpoint, body) {
  const shown = JSON.stringify({ ...body }, null, 2).replace(/("(?:prompt)"\s*:\s*")((?:[^"\\]|\\.)*)(")/g, '$1$2$3');
  return `curl ${endpoint} \\\n  -H "Authorization: Bearer $POKE_API_KEY" \\\n  -H "Content-Type: application/json" \\\n  -d '${shown.replace(/'/g, "'\\''")}'`;
}

function curlForMultipart(endpoint, model, prompt, files, hasMask, common, streamOn) {
  const lines = [
    `curl ${endpoint} \\`,
    `  -H "Authorization: Bearer $POKE_API_KEY" \\`,
    `  -F 'model=${model}' \\`,
    `  -F 'prompt=${prompt.replace(/'/g, "'\\''")}' \\`,
  ];
  const field = files.length > 1 ? 'image[]' : 'image';
  files.forEach(f => lines.push(`  -F '${field}=@${f.name};type=${f.type || 'image/png'}' \\`));
  if (hasMask) lines.push(`  -F 'mask=@mask.png;type=image/png' \\`);
  for (const [k, v] of Object.entries(common)) {
    if (k === 'stream') continue;
    lines.push(`  -F '${k}=${typeof v === 'object' ? JSON.stringify(v) : v}' \\`);
  }
  if (streamOn) lines.push(`  -F 'stream=true' \\`);
  return lines.join('\n').replace(/ \\\n$/, '');
}

/* ---------- 请求执行 ---------- */
let requestSeq = 0;

async function runRequest(mode) {
  const seq = ++requestSeq;
  let req;
  try { req = mode === 'gen' ? buildGenRequest() : buildEditRequest(); }
  catch (err) { status(err.message, 'error'); return; }

  $('btnToggleReqLog').dataset.hasReq = '1';
  $('curlPreview').textContent = req.describe;
  $('bodyPreview').textContent = req.body ? JSON.stringify(req.body, null, 2) : '（multipart/form-data，见上方 cURL）';
  $('respPreview').textContent = '…';

  const streaming = req.init.body instanceof FormData
    ? req.streamFlag
    : Boolean(req.body?.stream);

  status(streaming ? '<span class="spin">◌</span> 流式生成中…' : '<span class="spin">◌</span> 生成中…', 'info');

  const t0 = performance.now();
  try {
    const resp = await fetch(req.endpoint, req.init);
    if (seq !== requestSeq) return;
    const contentType = resp.headers.get('content-type') || '';

    if (!resp.ok) {
      const text = await resp.text();
      let msg = `HTTP ${resp.status}`;
      try {
        const j = JSON.parse(text);
        if (j.error) msg += ` · ${j.error.message || j.error.code || ''} (${j.error.type || 'error'})`;
      } catch { if (text) msg += ` · ${text.slice(0, 300)}`; }
      $('respPreview').textContent = text || '(空响应)';
      status(`❌ ${msg}`, 'error');
      return;
    }

    if (streaming || contentType.includes('text/event-stream')) {
      await consumeStream(resp, seq, mode);
    } else {
      const json = await resp.json();
      $('respPreview').textContent = JSON.stringify(json, null, 2).slice(0, 200000);
      const images = (json.data ?? [])
        .map(d => d.b64_json ? { b64: d.b64_json } : d.url ? { url: d.url } : null)
        .filter(Boolean);
      if (!images.length) {
        status('⚠️ 响应中没有图片数据，详见「请求详情」', 'error');
        return;
      }
      addHistory({ mode, model: (req.body?.model) || '(multipart)', prompt: (req.body?.prompt) || $('editPrompt').value, usage: json.usage, images });
      status(`✅ 完成 · ${images.length} 张 · ${((performance.now() - t0) / 1000).toFixed(1)}s${usageText(json.usage)}`, 'ok');
    }
  } catch (err) {
    if (seq !== requestSeq) return;
    const hint = err instanceof TypeError && err.message === 'Failed to fetch'
      ? '（可能是浏览器 CORS 拦截：目标网关未放行本页来源。可在 Provider 里配置代理前缀，或用支持 CORS 的网关）'
      : '';
    status(`❌ 请求失败：${err.message} ${hint}`, 'error');
  }
}

/* SSE 流式消费：image_generation.partial_image / image_generation.completed 等事件统一按 b64_json 提取 */
async function consumeStream(resp, seq, mode) {
  const reader = resp.body.getReader();
  const decoder = new TextDecoder();
  let buf = '';
  const partials = new Map();
  let final = null;
  let usage = null;
  let revised = null;

  const renderPartials = () => {
    const sorted = [...partials.entries()].sort((a, b) => a[0] - b[0]).map(([, v]) => v);
    if (sorted.length) {
      status(`<span class="spin">◌</span> 流式生成中 · 已收到 ${partials.size} 帧渐进图${final ? ' · 已完成' : ''}`, 'info');
      renderHistoryItem(null, sorted.map(b64 => ({ b64, partial: !final })), final ? [] : null);
    }
  };

  while (true) {
    const { done, value } = await reader.read();
    if (done) break;
    buf = (buf + decoder.decode(value, { stream: true })).replace(/\r\n/g, '\n');
    let idx;
    while ((idx = buf.indexOf('\n\n')) >= 0) {
      const chunk = buf.slice(0, idx);
      buf = buf.slice(idx + 2);
      for (const rawLine of chunk.split('\n')) {
        const line = rawLine.trim();
        if (!line.startsWith('data:')) continue;
        const payload = line.slice(5).trim();
        if (!payload || payload === '[DONE]') continue;
        let evt;
        try { evt = JSON.parse(payload); } catch { continue; }
        if (evt.usage) usage = evt.usage;
        if (evt.revised_prompt) revised = evt.revised_prompt;
        const isPartial = String(evt.type || '').includes('partial') || typeof evt.partial_image_index === 'number';
        if (evt.b64_json) {
          if (isPartial && !final) {
            partials.set(evt.partial_image_index ?? partials.size, evt.b64_json);
            renderPartials();
          } else {
            final = { b64: evt.b64_json };
          }
        }
        if (seq !== requestSeq) { try { reader.cancel(); } catch {} return; }
      }
    }
  }

  const images = final ? [final] : [...partials.values()].slice(-1).map(b64 => ({ b64 }));
  if (!images.length) {
    status('⚠️ 流式响应中没有图片数据，详见「请求详情」', 'error');
    return;
  }
  addHistory({ mode, model: mode === 'gen' ? resolveModel('gen') : resolveModel('edit'), prompt: mode === 'gen' ? $('genPrompt').value : $('editPrompt').value, usage, images, revised });
  status(`✅ 流式完成 · ${images.length} 张 · ${partials.size} 帧渐进${usageText(usage)}`, 'ok');
}

function usageText(u) {
  if (!u) return '';
  const parts = [];
  if (u.total_tokens != null) parts.push(`tokens ${u.total_tokens}`);
  if (u.input_tokens_details?.image_tokens != null) parts.push(`图入 ${u.input_tokens_details.image_tokens}`);
  if (u.output_tokens != null) parts.push(`出 ${u.output_tokens}`);
  return parts.length ? ` · ${parts.join(' / ')}` : '';
}

/* ---------- 历史 ---------- */
function addHistory(entry) {
  state.lastImages = entry.images;
  state.history.unshift({ id: uid(), ts: Date.now(), ...entry });
  if (state.history.length > 20) state.history.length = 20;
  renderHistory();
}

function renderHistory() {
  const list = $('historyList');
  list.innerHTML = '';
  if (!state.history.length) {
    list.innerHTML = '<div class="empty-tip">还没有结果。填好参数后点「生成图片」试试。</div>';
    return;
  }
  state.history.forEach(h => renderHistoryItem(h, h.images, null));
}

function renderHistoryItem(h, images, _unused) {
  const list = $('historyList');
  const box = document.createElement('div');
  box.className = 'history-item';
  const meta = h ? `
    <div class="history-meta">
      <span class="tag">${h.mode === 'gen' ? '文生图' : '编辑'}</span>
      <span>${new Date(h.ts).toLocaleTimeString()}</span>
      <span>${escapeHTML(h.model || '')}</span>
      ${h.usage?.total_tokens != null ? `<span>tokens ${h.usage.total_tokens}</span>` : ''}
      ${h.revised ? `<span class="hint">revised_prompt: ${escapeHTML(h.revised.slice(0, 60))}…</span>` : ''}
    </div>
    <div class="hint" style="margin-bottom:8px">${escapeHTML((h.prompt || '').slice(0, 200))}</div>` : '';
  box.innerHTML = meta + '<div class="history-images"></div>';
  const grid = box.querySelector('.history-images');
  images.forEach((img, i) => {
    const src = img.b64 ? `data:image/png;base64,${img.b64}` : img.url;
    const wrap = document.createElement('div');
    wrap.className = 'result-img';
    wrap.innerHTML = `<img alt="结果 ${i + 1}"><button class="dl">下载</button>${img.partial ? '<span class="partial-note">渐进帧</span>' : ''}`;
    wrap.querySelector('img').src = src;
    wrap.addEventListener('click', (e) => {
      if (e.target.classList.contains('dl')) { downloadImage(src, i); return; }
      openViewer(src, i);
    });
    grid.appendChild(wrap);
  });
  list.prepend(box);
}

async function downloadImage(src, i) {
  const name = `poke-image-${Date.now()}-${i + 1}.png`;
  try {
    if (src.startsWith('data:')) {
      triggerDownload(src, name);
    } else {
      const blob = await (await fetch(src)).blob();
      triggerDownload(URL.createObjectURL(blob), name);
    }
  } catch { window.open(src, '_blank'); }
}

function triggerDownload(href, name) {
  const a = document.createElement('a');
  a.href = href; a.download = name;
  document.body.appendChild(a); a.click(); a.remove();
}

function openViewer(src, i) {
  $('viewerImg').src = src;
  $('viewerDownload').href = src;
  $('viewerDownload').download = `poke-image-${Date.now()}-${i + 1}.png`;
  openModal('viewerModal');
}

/* ---------- 状态条 ---------- */
function status(html, type = 'info') {
  const el = $('statusBar');
  el.innerHTML = html;
  el.className = `status ${type}`;
  el.hidden = false;
}

/* ---------- 编辑图片上传 ---------- */
function addEditFiles(fileList) {
  for (const f of fileList) {
    if (!f.type.startsWith('image/')) continue;
    state.editFiles.push({ file: f, thumbUrl: URL.createObjectURL(f) });
  }
  if (state.editFiles.length > 16) {
    state.editFiles.length = 16;
    status('最多 16 张参考图，已截断', 'error');
  }
  renderEditThumbs();
}

function renderEditThumbs() {
  const box = $('imageThumbs');
  box.innerHTML = '';
  state.editFiles.forEach((x, i) => {
    const d = document.createElement('div');
    d.className = 'thumb';
    d.innerHTML = `<img alt="参考图 ${i + 1}"><button class="rm" title="移除">✕</button>`;
    d.querySelector('img').src = x.thumbUrl;
    d.querySelector('.rm').addEventListener('click', () => {
      URL.revokeObjectURL(x.thumbUrl);
      state.editFiles.splice(i, 1);
      renderEditThumbs();
    });
    box.appendChild(d);
  });
}

async function addLastResultAsReference() {
  const last = state.lastImages[0];
  if (!last) { status('还没有可用的最近结果', 'error'); return; }
  try {
    const src = last.b64 ? `data:image/png;base64,${last.b64}` : last.url;
    const blob = await (await fetch(src)).blob();
    const file = new File([blob], `result-${Date.now()}.png`, { type: blob.type || 'image/png' });
    addEditFiles([file]);
    switchMode('edit');
    status('已把最近结果加入参考图', 'ok');
  } catch (err) { status('加入失败：' + err.message, 'error'); }
}

/* ---------- 蒙版画板 ---------- */
const maskPainter = {
  baseImg: null,   // HTMLImageElement
  w: 1024, h: 1024,
  tool: 'brush',
  drawing: false,
  undoStack: [],
};

function openMaskPainter() {
  openModal('maskModal');
  if (!maskPainter.initialized) { initMaskCanvas(); maskPainter.initialized = true; }
}

function initMaskCanvas() {
  const paint = $('maskPaint');
  paint.width = maskPainter.w;
  paint.height = maskPainter.h;
  const ctx = paint.getContext('2d');
  ctx.lineCap = ctx.lineJoin = 'round';
  fitMaskWrap();
}

function fitMaskWrap() {
  $('maskCanvasWrap').style.aspectRatio = `${maskPainter.w} / ${maskPainter.h}`;
}

async function loadMaskBase(src) {
  const img = new Image();
  img.crossOrigin = 'anonymous';
  await new Promise((res, rej) => { img.onload = res; img.onerror = () => rej(new Error('图片加载失败')); img.src = src; });
  maskPainter.baseImg = img;
  const scale = Math.min(1, 2048 / Math.max(img.naturalWidth, img.naturalHeight));
  maskPainter.w = Math.round(img.naturalWidth * scale);
  maskPainter.h = Math.round(img.naturalHeight * scale);
  const base = $('maskBase');
  base.width = maskPainter.w; base.height = maskPainter.h;
  base.getContext('2d').drawImage(img, 0, 0, maskPainter.w, maskPainter.h);
  const paint = $('maskPaint');
  paint.width = maskPainter.w; paint.height = maskPainter.h;
  paint.getContext('2d').lineCap = paint.getContext('2d').lineJoin = 'round';
  $('maskSizeHint').textContent = `底图 ${img.naturalWidth}x${img.naturalHeight} · 蒙版输出 ${maskPainter.w}x${maskPainter.h}`;
  fitMaskWrap();
}

function pushUndo() {
  const paint = $('maskPaint');
  const ctx = paint.getContext('2d');
  maskPainter.undoStack.push(ctx.getImageData(0, 0, paint.width, paint.height));
  if (maskPainter.undoStack.length > 15) maskPainter.undoStack.shift();
}

function maskPointerPos(e) {
  const rect = $('maskPaint').getBoundingClientRect();
  return {
    x: (e.clientX - rect.left) * (maskPainter.w / rect.width),
    y: (e.clientY - rect.top) * (maskPainter.h / rect.height),
  };
}

function maskStroke(from, to) {
  const ctx = $('maskPaint').getContext('2d');
  ctx.globalCompositeOperation = maskPainter.tool === 'eraser' ? 'destination-out' : 'source-over';
  ctx.strokeStyle = 'rgba(255,96,96,0.9)';
  ctx.lineWidth = Number($('maskBrushSize').value);
  ctx.beginPath();
  ctx.moveTo(from.x, from.y);
  ctx.lineTo(to.x, to.y);
  ctx.stroke();
}

async function applyMaskFromPainter() {
  const paint = $('maskPaint');
  const out = document.createElement('canvas');
  out.width = paint.width; out.height = paint.height;
  const ctx = out.getContext('2d');
  // 先铺满不透明，再把涂刷区域挖成透明（= 重绘区）；反相则相反
  ctx.fillStyle = '#000';
  ctx.fillRect(0, 0, out.width, out.height);
  ctx.globalCompositeOperation = state.maskInvert ? 'source-over' : 'destination-out';
  ctx.drawImage(paint, 0, 0);
  const blob = await new Promise(res => out.toBlob(res, 'image/png'));
  const dataUrl = out.toDataURL('image/png');
  setEditMask(blob, dataUrl, `画板${state.maskInvert ? '·反相' : ''} ${out.width}x${out.height}`);
  closeModal('maskModal');
  status('蒙版已应用，将随下一次编辑请求上传', 'ok');
}

function setEditMask(blob, dataUrl, label) {
  state.editMask = { blob, dataUrl, label };
  const thumb = $('maskThumb');
  thumb.hidden = false;
  thumb.innerHTML = `<img alt="蒙版预览" title="${escapeHTML(label)}"><span class="partial-note">${escapeHTML(label)}</span>`;
  thumb.querySelector('img').src = dataUrl;
}

function clearEditMask() {
  state.editMask = null;
  $('maskThumb').hidden = true;
}

/* ---------- 模型元数据配置（per-Provider 覆盖） ---------- */
const METADATA_TEMPLATES = {
  'gpt-image-2.5（xhigh/max + 任意尺寸 + 流式）': () => ({
    stream: true, maxPrompt: 32000,
    editsText: '支持：mask + 最多 16 张参考图 + input_fidelity',
    params: gptCommonParams(['auto', 'high', 'medium', 'low', 'xhigh', 'max'], true),
  }),
  '网关通用（prompt/size/quality/n）': () => ({
    stream: false, maxPrompt: 32000,
    editsText: '以网关为准',
    params: [
      PARAM_LIB.quality(['auto', 'high', 'medium', 'low']),
      PARAM_LIB.size(['auto', '1024x1024', '1536x1024', '1024x1536'], false, ''),
      PARAM_LIB.n(10),
      PARAM_LIB.user(),
    ],
  }),
  'Gemini / Banana 风格（示例，可改字段名）': () => ({
    stream: false, maxPrompt: 32000,
    editsText: '指令式编辑（多参考图，无 mask）',
    params: [
      { key: 'image_size', label: 'image_size 分辨率档', type: 'select', options: ['1K', '2K', '4K'] },
      { key: 'aspect_ratio', label: 'aspect_ratio 比例', type: 'select', options: ['auto', '1:1', '3:4', '2:3', '4:3', '16:9', '9:16'] },
      PARAM_LIB.n(4),
      PARAM_LIB.user(),
    ],
  }),
};

function openModelConfig(modelId) {
  const p = activeProvider();
  const prof = resolveProfile(modelId);
  $('mCfgSubtitle').textContent = `${modelId} · ${p.name}`;
  const ov = p.modelConfigs?.[modelId];
  const meta = ov || { stream: prof.stream, maxPrompt: prof.maxPrompt, editsText: prof.editsText, params: prof.params };
  $('mCfgJson').value = JSON.stringify(meta, null, 2);
  $('mCfgTemplate').innerHTML = Object.keys(METADATA_TEMPLATES).map(k => `<option>${escapeHTML(k)}</option>`).join('');
  $('mCfgStatus').textContent = '';
  $('modelCfgModal').dataset.model = modelId;
  openModal('modelCfgModal');
}

function validateMetadata(meta) {
  if (typeof meta !== 'object' || !meta || Array.isArray(meta)) return '元数据必须是 JSON 对象';
  if (meta.stream !== undefined && typeof meta.stream !== 'boolean') return 'stream 必须是布尔值';
  if (meta.maxPrompt !== undefined && (!Number.isInteger(meta.maxPrompt) || meta.maxPrompt < 1)) return 'maxPrompt 需为正整数';
  if (meta.params !== undefined) {
    if (!Array.isArray(meta.params)) return 'params 必须是数组';
    const seen = new Set();
    for (const d of meta.params) {
      if (!d || typeof d.key !== 'string' || !d.key) return 'params 项缺少 key（发送到请求体的字段名）';
      if (seen.has(d.key)) return `params 重复 key：${d.key}`;
      seen.add(d.key);
      if (!['select', 'number', 'text', 'size'].includes(d.type)) return `param「${d.key}」type 不支持（可选 select / number / text / size）`;
      if ((d.type === 'select' || d.type === 'size') && !Array.isArray(d.options)) return `param「${d.key}」需要 options 数组`;
    }
  }
  return null;
}

function saveModelConfig() {
  const modelId = $('modelCfgModal').dataset.model;
  let meta;
  try { meta = JSON.parse($('mCfgJson').value); }
  catch (e) { $('mCfgStatus').textContent = '✗ JSON 解析失败：' + e.message; return; }
  const err = validateMetadata(meta);
  if (err) { $('mCfgStatus').textContent = '✗ ' + err; return; }
  const p = activeProvider();
  p.modelConfigs = p.modelConfigs || {};
  p.modelConfigs[modelId] = meta;
  saveState();
  refreshModelSidebar();
  closeModal('modelCfgModal');
  status(`✅ 已应用 ${modelId} 的元数据配置（仅 Provider「${escapeHTML(p.name)}」），参数区已按其重新生成`, 'ok');
}

function resetModelConfig() {
  const modelId = $('modelCfgModal').dataset.model;
  const p = activeProvider();
  if (!p?.modelConfigs?.[modelId]) return;
  delete p.modelConfigs[modelId];
  saveState();
  refreshModelSidebar();
  closeModal('modelCfgModal');
  status(`已重置 ${modelId} 为内置档案`, 'ok');
}

/* ---------- 弹窗 ---------- */
function openModal(id) { $(id).hidden = false; }
function closeModal(id) { $(id).hidden = true; }

/* ---------- 表单状态持久化 ---------- */
function snapshotForm() {
  const data = {};
  for (const section of ['genSection', 'editSection']) {
    $(section).querySelectorAll('input, select, textarea').forEach(el => {
      if (!el.id) return;
      data[el.id] = el.type === 'checkbox' ? el.checked : el.value;
    });
  }
  try { localStorage.setItem(LS_KEY + ':form', JSON.stringify(data)); } catch {}
}

function restoreForm() {
  let data;
  try { data = JSON.parse(localStorage.getItem(LS_KEY + ':form') || '{}'); } catch { return; }
  for (const [id, val] of Object.entries(data)) {
    const el = $(id);
    if (!el) continue;
    if (el.type === 'checkbox') el.checked = val; else el.value = val;
  }
}

/* ---------- 模式切换 ---------- */
function switchMode(mode) {
  state.mode = mode;
  $('genSection').hidden = mode !== 'gen';
  $('editSection').hidden = mode !== 'edit';
  document.querySelectorAll('.tab').forEach(t => t.classList.toggle('active', t.dataset.mode === mode));
  renderModelList(); // 高亮跟随当前模式各自的模型选择
}

/* ---------- 工具 ---------- */
function escapeHTML(s) {
  return String(s).replace(/[&<>"']/g, c => ({ '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&#39;' }[c]));
}

function updateProxyHint() {
  const p = activeProvider();
  if (p?.proxy) status(`当前 Provider 启用代理前缀：${escapeHTML(p.proxy)}`, 'info');
}

/* ---------- 配置导入导出 ---------- */
function exportConfig() {
  const blob = new Blob([JSON.stringify({ providers: state.providers, activeProvider: state.activeProvider }, null, 2)], { type: 'application/json' });
  triggerDownload(URL.createObjectURL(blob), `poke-image-studio-config-${new Date().toISOString().slice(0, 10)}.json`);
}

function importConfig(file) {
  const reader = new FileReader();
  reader.onload = () => {
    try {
      const data = JSON.parse(reader.result);
      if (!Array.isArray(data.providers)) throw new Error('格式不对：缺少 providers 数组');
      state.providers = data.providers;
      state.activeProvider = Math.min(data.activeProvider ?? 0, data.providers.length - 1);
      saveState();
      renderProviders();
      refreshModelSidebar();
      status('配置已导入', 'ok');
    } catch (err) { status('导入失败：' + err.message, 'error'); }
  };
  reader.readAsText(file);
}

/* ---------- 事件绑定 ---------- */
function bindEvents() {
  // 顶栏
  $('providerQuickSelect').addEventListener('change', (e) => {
    state.activeProvider = +e.target.value;
    saveState(); renderProviders(); refreshModelSidebar(); updateProxyHint();
  });
  $('btnManageProvider').addEventListener('click', () => openProviderEditor(state.activeProvider));
  $('btnExportConfig').addEventListener('click', exportConfig);
  $('btnImportConfig').addEventListener('click', () => $('importConfigFile').click());
  $('importConfigFile').addEventListener('change', (e) => { if (e.target.files[0]) importConfig(e.target.files[0]); e.target.value = ''; });

  // Provider 编辑
  $('btnAddProvider').addEventListener('click', () => {
    state.providers.push({ name: '新 Provider', baseUrl: '', apiKey: '', proxy: '', models: [...GPT_IMAGE_MODELS] });
    saveState();
    renderProviders();
    refreshModelSidebar();
    openProviderEditor(state.providers.length - 1);
  });
  $('btnSaveProvider').addEventListener('click', saveProviderFromEditor);
  $('btnCancelProvider').addEventListener('click', closeProviderEditor);
  $('btnDeleteProvider').addEventListener('click', () => {
    const i = state.editingProvider;
    if (i < 0) return;
    if (!confirm(`删除 Provider「${state.providers[i].name}」？`)) return;
    state.providers.splice(i, 1);
    if (!state.providers.length) state.providers = defaultProviders();
    state.activeProvider = Math.min(state.activeProvider, state.providers.length - 1);
    saveState();
    renderProviders();
    refreshModelSidebar();
    closeProviderEditor();
  });
  $('btnFetchModels').addEventListener('click', fetchModels);
  $('btnToggleKey').addEventListener('click', () => {
    const el = $('pApiKey');
    const show = el.type === 'password';
    el.type = show ? 'text' : 'password';
    $('btnToggleKey').textContent = show ? '隐藏' : '显示';
  });

  // Tab
  document.querySelectorAll('.tab').forEach(t => t.addEventListener('click', () => switchMode(t.dataset.mode)));

  // 侧边栏模型列表：▸ 展开/收起，⚙ 元数据配置，其余区域点选模型
  $('modelList').addEventListener('click', (e) => {
    const cfgBtn = e.target.closest('.m-cfg');
    if (cfgBtn) { openModelConfig(cfgBtn.dataset.cfg); return; }
    const tg = e.target.closest('.m-toggle');
    if (tg) {
      const m = tg.dataset.toggle;
      if (collapsedModels.has(m)) collapsedModels.delete(m); else collapsedModels.add(m);
      renderModelList();
      return;
    }
    const item = e.target.closest('.model-item');
    if (item) pickModel(item.dataset.model);
  });
  // 模型元数据配置弹窗
  $('mCfgApplyTemplate').addEventListener('click', () => {
    const t = METADATA_TEMPLATES[$('mCfgTemplate').value];
    if (t) $('mCfgJson').value = JSON.stringify(t(), null, 2);
  });
  $('mCfgSave').addEventListener('click', saveModelConfig);
  $('mCfgReset').addEventListener('click', resetModelConfig);
  $('btnAddModel').addEventListener('click', () => {
    const input = $('newModelId');
    const id = input.value.trim();
    if (!id) return;
    const p = activeProvider();
    if (!p.models.includes(id)) p.models.push(id);
    saveState();
    pickModel(id);
    input.value = '';
    status(`已添加模型 ${id} 并设为当前模式模型`, 'ok');
  });
  $('newModelId').addEventListener('keydown', (e) => {
    if (e.key === 'Enter') { e.preventDefault(); $('btnAddModel').click(); }
  });

  // prompt 计数（上限随模型档案变化）
  for (const prefix of ['gen', 'edit']) {
    $(`${prefix}Prompt`).addEventListener('input', () => updatePromptCounter(prefix));
    updatePromptCounter(prefix);
  }

  // 生成 / 编辑
  $('btnGenerate').addEventListener('click', () => runRequest('gen'));
  $('btnEdit').addEventListener('click', () => runRequest('edit'));
  document.addEventListener('keydown', (e) => {
    if ((e.metaKey || e.ctrlKey) && e.key === 'Enter') {
      e.preventDefault();
      runRequest(state.mode);
    }
  });

  // 图片上传
  $('imageDrop').addEventListener('click', () => $('imageFiles').click());
  $('imageFiles').addEventListener('change', (e) => { addEditFiles(e.target.files); e.target.value = ''; });
  ['dragover', 'dragenter'].forEach(ev => $('imageDrop').addEventListener(ev, (e) => { e.preventDefault(); $('imageDrop').classList.add('drag'); }));
  ['dragleave', 'drop'].forEach(ev => $('imageDrop').addEventListener(ev, (e) => { e.preventDefault(); $('imageDrop').classList.remove('drag'); }));
  $('imageDrop').addEventListener('drop', (e) => { if (e.dataTransfer?.files?.length) addEditFiles(e.dataTransfer.files); });
  $('btnClearImages').addEventListener('click', () => { state.editFiles.forEach(x => URL.revokeObjectURL(x.thumbUrl)); state.editFiles = []; renderEditThumbs(); });
  $('btnAddLastResult').addEventListener('click', addLastResultAsReference);

  // 蒙版
  $('btnOpenMaskPainter').addEventListener('click', openMaskPainter);
  $('btnUploadMask').addEventListener('click', () => $('maskFile').click());
  $('maskFile').addEventListener('change', (e) => {
    const f = e.target.files[0];
    if (!f) return;
    const reader = new FileReader();
    reader.onload = () => setEditMask(f, reader.result, `文件 ${f.name}`);
    reader.readAsDataURL(f);
    e.target.value = '';
  });
  $('btnClearMask').addEventListener('click', clearEditMask);

  // 蒙版画板
  $('maskLoadRef').addEventListener('click', async () => {
    const first = state.editFiles[0];
    if (!first) { status('还没有参考图，先在编辑表单上传一张', 'error'); return; }
    try { await loadMaskBase(first.thumbUrl); } catch (err) { status(err.message, 'error'); }
  });
  $('maskLoadLast').addEventListener('click', async () => {
    const last = state.lastImages[0];
    if (!last) { status('还没有最近结果', 'error'); return; }
    try { await loadMaskBase(last.b64 ? `data:image/png;base64,${last.b64}` : last.url); }
    catch (err) { status(err.message, 'error'); }
  });
  $('maskLoadFile').addEventListener('change', (e) => {
    const f = e.target.files[0];
    if (f) loadMaskBase(URL.createObjectURL(f)).catch(err => status(err.message, 'error'));
    e.target.value = '';
  });
  $('maskBrush').addEventListener('click', () => { maskPainter.tool = 'brush'; $('maskBrush').classList.add('active-tool'); $('maskEraser').classList.remove('active-tool'); });
  $('maskEraser').addEventListener('click', () => { maskPainter.tool = 'eraser'; $('maskEraser').classList.add('active-tool'); $('maskBrush').classList.remove('active-tool'); });
  $('maskUndo').addEventListener('click', () => {
    const snap = maskPainter.undoStack.pop();
    if (snap) $('maskPaint').getContext('2d').putImageData(snap, 0, 0);
  });
  $('maskClear').addEventListener('click', () => {
    pushUndo();
    const ctx = $('maskPaint').getContext('2d');
    ctx.globalCompositeOperation = 'source-over';
    ctx.clearRect(0, 0, $('maskPaint').width, $('maskPaint').height);
  });
  $('maskInvert').addEventListener('click', () => {
    state.maskInvert = !state.maskInvert;
    $('maskInvert').classList.toggle('active-tool', state.maskInvert);
    $('maskSizeHint').textContent = state.maskInvert
      ? '反相开启：涂刷区保留、其余透明（涂刷区之外重绘）'
      : '默认：涂刷区导出为透明 = 重绘区域';
  });
  $('maskApply').addEventListener('click', applyMaskFromPainter);

  const paint = $('maskPaint');
  paint.addEventListener('pointerdown', (e) => {
    e.preventDefault();
    paint.setPointerCapture(e.pointerId);
    pushUndo();
    maskPainter.drawing = true;
    maskPainter.last = maskPointerPos(e);
    maskStroke(maskPainter.last, maskPainter.last);
  });
  paint.addEventListener('pointermove', (e) => {
    if (!maskPainter.drawing) return;
    const pos = maskPointerPos(e);
    maskStroke(maskPainter.last, pos);
    maskPainter.last = pos;
  });
  ['pointerup', 'pointercancel', 'pointerleave'].forEach(ev =>
    paint.addEventListener(ev, () => { maskPainter.drawing = false; })
  );

  // 历史 / 结果
  $('btnClearHistory').addEventListener('click', () => { state.history = []; state.lastImages = []; renderHistory(); });
  $('btnToggleReqLog').addEventListener('click', () => openModal('reqLogModal'));

  // 弹窗关闭
  document.querySelectorAll('[data-close]').forEach(btn =>
    btn.addEventListener('click', () => closeModal(btn.dataset.close))
  );
  document.querySelectorAll('.modal').forEach(m =>
    m.addEventListener('click', (e) => { if (e.target === m) closeModal(m.id); })
  );
  document.addEventListener('keydown', (e) => {
    if (e.key === 'Escape') document.querySelectorAll('.modal:not([hidden])').forEach(m => closeModal(m.id));
  });

  // 表单快照
  window.addEventListener('beforeunload', snapshotForm);
  setInterval(snapshotForm, 5000);
}

/* ---------- 启动 ---------- */
function init() {
  loadState();
  renderProviders();
  refreshModelSidebar();   // 渲染列表 + 按当前模型的元数据生成参数控件
  restoreForm();           // 恢复表单值（同模型时动态控件 id 一致，可直接恢复）
  updatePromptCounter('gen');
  updatePromptCounter('edit');
  updateSizeHint('gen');
  updateSizeHint('edit');
  renderHistory();
  bindEvents();
}

init();
