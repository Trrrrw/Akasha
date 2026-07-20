import { useEffect, useMemo } from "react";
import Markdown from "react-markdown";
import remarkGfm from "remark-gfm";

export type MarkdownHeadingLevel = 1 | 2 | 3 | 4 | 5 | 6;

export type MarkdownHeading = {
  id: string;
  label: string;
  level: MarkdownHeadingLevel;
};

type MarkdownContentProps = {
  markdown: string;
  onHeadingsChange?: (headings: MarkdownHeading[]) => void;
};

type HastNode = {
  type?: string;
  tagName?: string;
  value?: string;
  children?: HastNode[];
  properties?: Record<string, unknown>;
};

function buildUniqueHeadingId(title: string, counts: Map<string, number>) {
  const count = (counts.get(title) ?? 0) + 1;
  counts.set(title, count);

  return count === 1 ? title : `${title}-${count}`;
}

function extractHeadings(markdown: string): MarkdownHeading[] {
  const counts = new Map<string, number>();

  return markdown
    .split("\n")
    .map((line) => {
      const match = line.match(/^(#{1,6})\s+(.+?)\s*#*$/);

      if (!match) {
        return null;
      }

      const title = match[2].trim();
      const level = match[1].length as MarkdownHeadingLevel;

      return {
        id: buildUniqueHeadingId(title, counts),
        label: title,
        level,
      };
    })
    .filter((heading): heading is MarkdownHeading => heading !== null);
}

function hastNodeText(node: HastNode): string {
  if (node.type === "text") {
    return node.value ?? "";
  }

  return node.children?.map(hastNodeText).join("") ?? "";
}

function rehypeHeadingIds() {
  return (tree: HastNode) => {
    const counts = new Map<string, number>();

    function visit(node: HastNode) {
      const headingLevel = node.tagName?.match(/^h([1-6])$/)?.[1];

      if (headingLevel) {
        const title = hastNodeText(node).trim();

        if (title) {
          node.properties = {
            ...node.properties,
            id: buildUniqueHeadingId(title, counts),
          };
        }
      }

      node.children?.forEach(visit);
    }

    visit(tree);
  };
}

export function MarkdownContent({
  markdown,
  onHeadingsChange,
}: MarkdownContentProps) {
  const headings = useMemo(() => extractHeadings(markdown), [markdown]);

  useEffect(() => {
    onHeadingsChange?.(headings);
  }, [headings, onHeadingsChange]);

  return (
    <div className="min-w-0">
      <Markdown
        rehypePlugins={[rehypeHeadingIds]}
        remarkPlugins={[remarkGfm]}
        components={{
          h1({ children, id }) {
            return (
              <h1
                id={id}
                className="scroll-m-20 text-4xl font-extrabold tracking-tight lg:text-5xl"
              >
                {children}
              </h1>
            );
          },
          h2({ children, id }) {
            return (
              <h2
                id={id}
                className="mt-10 scroll-m-20 border-b pb-2 text-3xl font-semibold tracking-tight first:mt-0"
              >
                {children}
              </h2>
            );
          },
          h3({ children, id }) {
            return (
              <h3
                id={id}
                className="mt-8 scroll-m-20 text-2xl font-semibold tracking-tight"
              >
                {children}
              </h3>
            );
          },
          h4({ children, id }) {
            return (
              <h4
                id={id}
                className="mt-8 scroll-m-20 text-xl font-semibold tracking-tight"
              >
                {children}
              </h4>
            );
          },
          h5({ children, id }) {
            return (
              <h5
                id={id}
                className="mt-8 scroll-m-20 text-lg font-semibold tracking-tight"
              >
                {children}
              </h5>
            );
          },
          h6({ children, id }) {
            return (
              <h6
                id={id}
                className="mt-8 scroll-m-20 text-base font-semibold tracking-tight"
              >
                {children}
              </h6>
            );
          },
        p({ children }) {
          return (
            <p className="leading-7 [&:not(:first-child)]:mt-6">{children}</p>
          );
        },
        a({ children, href }) {
          return (
            <a
              href={href}
              className="font-medium text-primary underline underline-offset-4"
              rel="noreferrer"
              target={href?.startsWith("http") ? "_blank" : undefined}
            >
              {children}
            </a>
          );
        },
        blockquote({ children }) {
          return (
            <blockquote className="mt-6 border-l-2 pl-6 italic">
              {children}
            </blockquote>
          );
        },
        ul({ children }) {
          return (
            <ul className="my-6 ml-6 list-disc [&>li]:mt-2">{children}</ul>
          );
        },
        ol({ children }) {
          return (
            <ol className="my-6 ml-6 list-decimal [&>li]:mt-2">{children}</ol>
          );
        },
        table({ children }) {
          return (
            <div className="my-6 w-full overflow-y-auto">
              <table className="w-full">{children}</table>
            </div>
          );
        },
        tr({ children }) {
          return <tr className="m-0 border-t p-0 even:bg-muted">{children}</tr>;
        },
        th({ children }) {
          return (
            <th className="border px-4 py-2 text-left font-bold [&[align=center]]:text-center [&[align=right]]:text-right">
              {children}
            </th>
          );
        },
        td({ children }) {
          return (
            <td className="border px-4 py-2 text-left [&[align=center]]:text-center [&[align=right]]:text-right">
              {children}
            </td>
          );
        },
        code({ children, className }) {
          const isBlock = className?.startsWith("language-");

          if (isBlock) {
            return <code className={className}>{children}</code>;
          }

          return (
            <code className="relative rounded bg-muted px-[0.3rem] py-[0.2rem] font-mono text-sm font-semibold">
              {children}
            </code>
          );
        },
        pre({ children }) {
          return (
            <pre className="my-6 overflow-x-auto rounded-lg border bg-muted p-4">
              {children}
            </pre>
          );
        },
        img({ alt, src }) {
          return (
            <img
              alt={alt ?? ""}
              className="my-6 rounded-lg border"
              src={src ?? ""}
            />
          );
        },
        }}
      >
        {markdown}
      </Markdown>
    </div>
  );
}
