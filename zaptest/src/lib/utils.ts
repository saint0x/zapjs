import { type ClassValue, clsx } from "clsx";
import { twMerge } from "tailwind-merge";

export function cn(...inputs: ClassValue[]) {
  return twMerge(clsx(inputs));
}

// Syntax highlighting colors
const HIGHLIGHT_COLORS = {
  comment: '#6b7280',    // carbon-500
  string: '#34d399',     // emerald-400
  keyword: '#a78bfa',    // violet-400
  type: '#38bdf8',       // sky-400
  special: '#fb923c',    // zap-400 (orange)
  tag: '#f472b6',        // pink-400 (JSX tags)
  bracket: '#9ca3af',    // carbon-400 (< > / brackets)
  default: '#d1d5db',    // carbon-300
} as const;

interface Token {
  text: string;
  color: string;
}

const RUST_KEYWORDS = ['use', 'pub', 'async', 'fn', 'struct', 'let', 'Ok', 'Result', 'impl', 'await', 'derive', 'mod', 'self', 'Self', 'where', 'for', 'in', 'if', 'else', 'match', 'return', 'mut', 'ref', 'move', 'dyn', 'trait', 'type', 'enum', 'true', 'false'];
const TS_KEYWORDS = ['import', 'export', 'const', 'async', 'await', 'return', 'function', 'from', 'interface', 'type', 'let', 'var', 'if', 'else', 'new', 'this', 'class', 'extends', 'true', 'false', 'null', 'undefined', 'as'];
const RUST_TYPES = ['User', 'String', 'u64', 'u32', 'i32', 'i64', 'bool', 'Vec', 'Option', 'UserUpdate', 'HashMap', 'Box', 'Arc', 'Mutex'];
const TS_TYPES = ['User', 'UserUpdate', 'string', 'number', 'void', 'boolean', 'any', 'unknown', 'never', 'Promise', 'Array', 'Record', 'Partial'];

export function highlightCode(code: string, lang: 'rust' | 'typescript'): Token[] {
  const tokens: Token[] = [];
  const keywords = lang === 'rust' ? RUST_KEYWORDS : TS_KEYWORDS;
  const types = lang === 'rust' ? RUST_TYPES : TS_TYPES;

  let i = 0;
  while (i < code.length) {
    // Comments
    if (code.slice(i, i + 2) === '//') {
      const end = code.indexOf('\n', i);
      const commentEnd = end === -1 ? code.length : end;
      tokens.push({ text: code.slice(i, commentEnd), color: HIGHLIGHT_COLORS.comment });
      i = commentEnd;
      continue;
    }

    // Strings (single quotes)
    if (code[i] === "'") {
      let j = i + 1;
      while (j < code.length && code[j] !== "'" && code[j] !== '\n') {
        if (code[j] === '\\') j++;
        j++;
      }
      tokens.push({ text: code.slice(i, j + 1), color: HIGHLIGHT_COLORS.string });
      i = j + 1;
      continue;
    }

    // Strings (double quotes)
    if (code[i] === '"') {
      let j = i + 1;
      while (j < code.length && code[j] !== '"' && code[j] !== '\n') {
        if (code[j] === '\\') j++;
        j++;
      }
      tokens.push({ text: code.slice(i, j + 1), color: HIGHLIGHT_COLORS.string });
      i = j + 1;
      continue;
    }

    // Template strings
    if (code[i] === '`') {
      let j = i + 1;
      while (j < code.length && code[j] !== '`') {
        if (code[j] === '\\') j++;
        j++;
      }
      tokens.push({ text: code.slice(i, j + 1), color: HIGHLIGHT_COLORS.string });
      i = j + 1;
      continue;
    }

    // Rust attributes #[...]
    if (lang === 'rust' && code[i] === '#' && code[i + 1] === '[') {
      let j = i + 2;
      let depth = 1;
      while (j < code.length && depth > 0) {
        if (code[j] === '[') depth++;
        if (code[j] === ']') depth--;
        j++;
      }
      tokens.push({ text: code.slice(i, j), color: HIGHLIGHT_COLORS.special });
      i = j;
      continue;
    }

    // server.* calls
    if (code.slice(i, i + 7) === 'server.') {
      let j = i + 7;
      while (j < code.length && /[\w.]/.test(code[j])) j++;
      tokens.push({ text: code.slice(i, j), color: HIGHLIGHT_COLORS.special });
      i = j;
      continue;
    }

    // @zap-js/...
    if (code.slice(i, i + 8) === '@zap-js/') {
      let j = i + 7;
      while (j < code.length && /\w/.test(code[j])) j++;
      tokens.push({ text: code.slice(i, j), color: HIGHLIGHT_COLORS.special });
      i = j;
      continue;
    }

    // JSX closing tag </tag>
    if (lang === 'typescript' && code[i] === '<' && code[i + 1] === '/') {
      tokens.push({ text: '</', color: HIGHLIGHT_COLORS.bracket });
      i += 2;
      // Get tag name
      let j = i;
      while (j < code.length && /[\w]/.test(code[j])) j++;
      if (j > i) {
        tokens.push({ text: code.slice(i, j), color: HIGHLIGHT_COLORS.tag });
        i = j;
      }
      // Get closing >
      if (code[i] === '>') {
        tokens.push({ text: '>', color: HIGHLIGHT_COLORS.bracket });
        i++;
      }
      continue;
    }

    // JSX opening tag <tag or self-closing <tag />
    if (lang === 'typescript' && code[i] === '<' && /[a-zA-Z]/.test(code[i + 1])) {
      tokens.push({ text: '<', color: HIGHLIGHT_COLORS.bracket });
      i++;
      // Get tag name
      let j = i;
      while (j < code.length && /[\w]/.test(code[j])) j++;
      tokens.push({ text: code.slice(i, j), color: HIGHLIGHT_COLORS.tag });
      i = j;
      continue;
    }

    // JSX self-closing /> or just >
    if (lang === 'typescript' && code[i] === '/' && code[i + 1] === '>') {
      tokens.push({ text: '/>', color: HIGHLIGHT_COLORS.bracket });
      i += 2;
      continue;
    }

    // Words (identifiers, keywords, types)
    if (/[a-zA-Z_]/.test(code[i])) {
      let j = i;
      while (j < code.length && /[\w]/.test(code[j])) j++;
      const word = code.slice(i, j);

      if (keywords.includes(word)) {
        tokens.push({ text: word, color: HIGHLIGHT_COLORS.keyword });
      } else if (types.includes(word)) {
        tokens.push({ text: word, color: HIGHLIGHT_COLORS.type });
      } else {
        tokens.push({ text: word, color: HIGHLIGHT_COLORS.default });
      }
      i = j;
      continue;
    }

    // Everything else (operators, whitespace, etc.)
    tokens.push({ text: code[i], color: HIGHLIGHT_COLORS.default });
    i++;
  }

  return tokens;
}

export function tokensToHtml(tokens: Token[]): string {
  return tokens
    .map(t => {
      const escaped = t.text
        .replace(/&/g, '&amp;')
        .replace(/</g, '&lt;')
        .replace(/>/g, '&gt;');
      return `<span style="color:${t.color}">${escaped}</span>`;
    })
    .join('');
}
