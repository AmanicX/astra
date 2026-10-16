import { memo, useState, type ReactNode } from "react";
import ReactMarkdown from "react-markdown";
import remarkGfm from "remark-gfm";
import rehypeHighlight from "rehype-highlight";
import { openUrl } from "@tauri-apps/plugin-opener";

function CodeBlock({ children, className }: { children?: ReactNode; className?: string }) {
  const [copied, setCopied] = useState(false);
  const raw = extractText(children);
  const lang = /language-(\w+)/.exec(className ?? "")?.[1];

  return (
    <div className="code-block">
      <div className="code-head">
        <span className="code-lang mono">{lang ?? "text"}</span>
        <button
          type="button"
          className="code-copy"
          onClick={() => {
            navigator.clipboard?.writeText(raw).then(
              () => {
                setCopied(true);
                setTimeout(() => setCopied(false), 1400);
              },
              () => {},
            );
          }}
        >
          {copied ? "Copied" : "Copy"}
        </button>
      </div>
      <pre>{children}</pre>
    </div>
  );
}

function extractText(node: ReactNode): string {
  if (node == null || typeof node === "boolean") return "";
  if (typeof node === "string" || typeof node === "number") return String(node);
  if (Array.isArray(node)) return node.map(extractText).join("");
  const el = node as { props?: { children?: ReactNode } };
  return el.props ? extractText(el.props.children) : "";
}

export const Markdown = memo(function Markdown({ children }: { children: string }) {
  return (
    <div className="prose">
      <ReactMarkdown
        remarkPlugins={[remarkGfm]}
        rehypePlugins={[[rehypeHighlight, { detect: true, ignoreMissing: true }]]}
        components={{
          a: ({ href, children: kids }) => (
            <a
              href={href}
              onClick={(e) => {
                e.preventDefault();
                if (href) openUrl(href).catch(() => {});
              }}
            >
              {kids}
            </a>
          ),
          pre: ({ children: kids, ...props }) => (
            <CodeBlock {...(props as { className?: string })}>{kids}</CodeBlock>
          ),
        }}
      >
        {children}
      </ReactMarkdown>
    </div>
  );
});
