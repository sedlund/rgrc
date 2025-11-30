# 自实现 Regex 增强功能可行性分析

## 问题背景

当前实现使用混合正则引擎：
- **regex** (快速，81% 配置文件): 标准 PCRE
- **fancy-regex** (功能完整，19% 配置文件): 支持 lookahead/lookbehind

目标：**移除 fancy-regex 依赖**，自己实现少量增强功能。

## 使用场景统计

### Lookahead 模式分析

```
最常见的 lookahead 模式：
4x  (?=\s|$)              - 匹配空格或行尾
4x  (?=\s[A-Z][a-z]{2}\s) - 匹配日期前的位置（ls 命令）
4x  (?=[\s,]|$)           - 匹配空格/逗号或行尾
2x  (?=\s{2,})            - 匹配两个以上空格
2x  (?=[-r][-w][-xsStT]...) - 匹配文件权限模式
```

### Lookbehind 模式分析

```
最常见的 lookbehind 模式：
15x (?<=\s)               - 匹配空格之后
4x  (?<=[-bcCdDlMnpPs?])  - 匹配文件类型字符之后
2x  (?<=[,<])             - 匹配逗号或尖括号之后
```

### 关键发现

1. **大部分是简单的边界匹配**
   - `(?=\s|$)` → 确保后面是空格或结尾
   - `(?<=\s)` → 确保前面是空格

2. **不需要复杂的回溯**
   - 没有递归模式
   - 没有动态长度的 lookbehind
   - 没有嵌套的 lookaround

3. **模式相对固定**
   - 大多数是字符类或简单字符串
   - 很少有复杂的子表达式

## 可行性分析

### 方案 1: 后处理验证法 ⭐️ **推荐**

**核心思路**: 先用 `regex` 匹配，然后手动验证 lookahead/lookbehind 条件

```rust
// 伪代码示例
fn match_with_lookahead(
    text: &str,
    main_pattern: &Regex,
    lookahead: &str,
    lookahead_regex: &Regex
) -> Option<Match> {
    for mat in main_pattern.find_iter(text) {
        let end_pos = mat.end();
        // 检查 lookahead: 从匹配结束位置开始检查
        if end_pos < text.len() && lookahead_regex.is_match(&text[end_pos..]) {
            return Some(mat);
        }
    }
    None
}

fn match_with_lookbehind(
    text: &str,
    main_pattern: &Regex,
    lookbehind: &str,
    lookbehind_regex: &Regex
) -> Option<Match> {
    for mat in main_pattern.find_iter(text) {
        let start_pos = mat.start();
        // 检查 lookbehind: 从匹配开始位置向前检查
        if start_pos > 0 && lookbehind_regex.is_match(&text[..start_pos]) {
            return Some(mat);
        }
    }
    None
}
```

**优点**:
- ✅ 实现简单（~100-200 行代码）
- ✅ 复用 `regex` crate 的高性能
- ✅ 只需支持有限的 lookaround 模式
- ✅ 可以渐进式实现

**缺点**:
- ⚠️ 需要解析正则表达式，提取 lookaround 部分
- ⚠️ 对于复杂模式（嵌套、回溯）较难处理

### 方案 2: 模式改写法

**核心思路**: 将 lookahead/lookbehind 改写为标准正则，调整捕获组

```rust
// 原始: (?<=\s)\d+(?=\s|$)
// 改写: (\s)(\d+)(\s|$)
// 然后只返回第 2 个捕获组
```

**优点**:
- ✅ 完全使用标准 regex
- ✅ 性能最优

**缺点**:
- ❌ 需要修改 84 个配置文件
- ❌ 维护成本高
- ❌ 可能破坏现有配置

### 方案 3: 模式预编译转换

**核心思路**: 在加载配置时，自动转换 lookaround 模式

```rust
fn convert_lookaround_pattern(pattern: &str) -> (String, Vec<CaptureAdjustment>) {
    // 检测 (?=...) 和 (?<=...)
    // 自动改写为 (...) 并记录需要调整的捕获组
    // 返回: (新模式, 捕获组映射)
}
```

**优点**:
- ✅ 不修改配置文件
- ✅ 对用户透明

**缺点**:
- ⚠️ 需要复杂的正则解析器
- ⚠️ 可能无法处理所有边界情况

## 实现建议

### 推荐方案：后处理验证法 + 渐进式实现

#### 阶段 1: 支持最常见的模式（覆盖 80%）

```rust
pub enum SimpleLookaround {
    Ahead(String),      // (?=pattern)
    Behind(String),     // (?<=pattern)
    NegAhead(String),   // (?!pattern)
    NegBehind(String),  // (?<!pattern)
}

pub struct EnhancedRegex {
    main_pattern: regex::Regex,
    lookaround: Option<SimpleLookaround>,
}

impl EnhancedRegex {
    pub fn new(pattern: &str) -> Result<Self, Error> {
        // 1. 检测是否包含 lookaround
        if let Some((main, lookaround)) = extract_simple_lookaround(pattern) {
            // 2. 编译主模式（移除 lookaround）
            let main_pattern = regex::Regex::new(&main)?;
            Ok(EnhancedRegex {
                main_pattern,
                lookaround: Some(lookaround),
            })
        } else {
            // 3. 没有 lookaround，直接编译
            Ok(EnhancedRegex {
                main_pattern: regex::Regex::new(pattern)?,
                lookaround: None,
            })
        }
    }
    
    pub fn captures_from_pos<'t>(&self, text: &'t str, pos: usize) 
        -> Option<Captures<'t>> 
    {
        let mat = self.main_pattern.captures_from_pos(text, pos)?;
        
        // 如果有 lookaround，验证条件
        if let Some(ref la) = self.lookaround {
            if !self.verify_lookaround(text, &mat, la) {
                // 不满足条件，继续搜索下一个
                return self.captures_from_pos(text, mat.get(0)?.end());
            }
        }
        
        Some(mat)
    }
    
    fn verify_lookaround(&self, text: &str, mat: &regex::Captures, 
                         la: &SimpleLookaround) -> bool {
        match la {
            SimpleLookaround::Ahead(pattern) => {
                let pos = mat.get(0).unwrap().end();
                if pos >= text.len() { return false; }
                // 简单匹配：检查后续文本是否匹配
                text[pos..].starts_with(pattern) || 
                    regex::Regex::new(pattern).unwrap().is_match(&text[pos..])
            }
            SimpleLookaround::Behind(pattern) => {
                let pos = mat.get(0).unwrap().start();
                if pos == 0 { return false; }
                // 简单匹配：检查前面的文本是否匹配
                text[..pos].ends_with(pattern) ||
                    regex::Regex::new(pattern).unwrap().is_match(&text[..pos])
            }
            SimpleLookaround::NegAhead(pattern) => {
                !self.verify_lookaround(text, mat, 
                    &SimpleLookaround::Ahead(pattern.clone()))
            }
            SimpleLookaround::NegBehind(pattern) => {
                !self.verify_lookaround(text, mat, 
                    &SimpleLookaround::Behind(pattern.clone()))
            }
        }
    }
}
```

#### 阶段 2: 正则解析器（处理复杂模式）

仅在简单提取失败时，使用更复杂的解析：

```rust
fn extract_simple_lookaround(pattern: &str) -> Option<(String, SimpleLookaround)> {
    // 处理最常见的情况
    if let Some(caps) = SIMPLE_LOOKAHEAD.captures(pattern) {
        let main = pattern.replace(&caps[0], "");
        return Some((main, SimpleLookaround::Ahead(caps[1].to_string())));
    }
    
    // 处理 lookbehind
    if let Some(caps) = SIMPLE_LOOKBEHIND.captures(pattern) {
        let main = pattern.replace(&caps[0], "");
        return Some((main, SimpleLookaround::Behind(caps[1].to_string())));
    }
    
    None // 回退到 fancy-regex
}

lazy_static! {
    // 匹配模式末尾的简单 lookahead
    static ref SIMPLE_LOOKAHEAD: Regex = 
        Regex::new(r"\(\?=([^)]+)\)$").unwrap();
    
    // 匹配模式开头的简单 lookbehind
    static ref SIMPLE_LOOKBEHIND: Regex = 
        Regex::new(r"^\(\?<=([^)]+)\)").unwrap();
}
```

## 代码量估算

```
src/enhanced_regex.rs     ~200 行  - 核心实现
src/lookaround_parser.rs  ~150 行  - 简单解析器
tests/enhanced_regex.rs   ~100 行  - 单元测试
------------------------------------------
总计:                     ~450 行
```

相比 fancy-regex (~8000 行代码 + 依赖)，这是一个很小的实现。

## 性能影响

### 最坏情况分析

```rust
// 原始 fancy-regex: O(n) 一次性匹配
// 新方案: O(n * m) 其中 m 是 false positive 数量

// 实际场景：
// - 大多数行只有 1-2 个匹配
// - lookaround 验证非常快（通常是字符比较）
// - 预期性能损失 < 5%
```

### 最佳情况

对于 81% 的配置文件（不需要 lookaround），性能保持不变（使用 Fast regex）。

## 风险评估

### 高风险
- ❌ **复杂嵌套 lookaround**: 如 `(?=(?!...)...)` 
  - 当前配置文件中未发现
  
- ❌ **变长 lookbehind**: 如 `(?<=\w+)`
  - 需要复杂的回溯
  - 当前仅有固定长度模式

### 低风险
- ✅ **简单边界匹配**: `(?=\s|$)`, `(?<=\s)`
  - 覆盖 ~70% 的使用场景
  
- ✅ **固定字符串**: `(?=\s[A-Z][a-z]{2}\s)`
  - 可以直接字符串比较

## 决策建议

### 建议：实施渐进式方案

#### 第 1 步: 实现 EnhancedRegex (1-2 天工作量)
- 支持最常见的 10 种模式
- 覆盖 ~80% 的 fancy-regex 使用场景
- 其余 20% 仍使用 fancy-regex 作为回退

#### 第 2 步: 测试和验证 (1 天)
- 运行现有 254 个测试
- 添加 20+ 个 EnhancedRegex 特定测试
- 性能基准测试

#### 第 3 步: 逐步移除 fancy-regex (可选)
- 如果 EnhancedRegex 通过所有测试
- 且性能损失 < 10%
- 可以完全移除 fancy-regex 依赖

### 收益分析

**优点**:
- ✅ 减少依赖: 移除 fancy-regex (~8KB 编译产物)
- ✅ 更快编译: 减少依赖编译时间
- ✅ 更好理解: 代码更简单，易于维护
- ✅ 潜在性能提升: 针对性优化常见模式

**缺点**:
- ⚠️ 开发成本: ~2-3 天工作量
- ⚠️ 维护成本: 需要维护新代码
- ⚠️ 兼容性风险: 可能有边界情况未覆盖

## 结论

**可行性: ⭐️⭐️⭐️⭐️ (4/5 星)**

基于以下理由：
1. **使用场景简单**: 仅需支持 ~10 种固定模式
2. **覆盖率高**: 80% 的 lookaround 使用场景很简单
3. **代码量少**: ~450 行代码即可实现
4. **风险可控**: 可以保留 fancy-regex 作为回退
5. **性能影响小**: 预计 < 5% 性能损失，甚至可能更快

**建议**: 
- ✅ **值得尝试**，作为下一阶段优化
- ✅ 采用渐进式策略，保留回退方案
- ✅ 优先实现最常见的 10 种模式
- ⚠️ 不建议一次性移除 fancy-regex，应逐步验证
