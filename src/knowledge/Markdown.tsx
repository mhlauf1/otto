import { useEffect, useId, useState } from 'react';
import ReactMarkdown, { type Components } from 'react-markdown';
import remarkGfm from 'remark-gfm';
import type { Area } from '../bindings';
import { useKnowledgeImage } from '../app/queries';
import { font } from '../design/tokens';
import { resolveImage } from './paths';
import styles from './Knowledge.module.css';

interface MarkdownProps {
  text: string;
  /** Where the note lives, for resolving its images. Omit for project docs. */
  source?: { workspaceId: string; area: Area; path: string } | undefined;
}

/**
 * Notes rendered as Markdown (GitHub flavour). Raw HTML is never rendered, links never
 * navigate the window, and ```mermaid blocks become diagrams.
 */
export function Markdown({ text, source }: MarkdownProps) {
  const components: Components = {
    // The page owns the only <h1>; a note's headings sit one level below it.
    h1: ({ children }) => <h2 data-md="1">{children}</h2>,
    h2: ({ children }) => <h3 data-md="2">{children}</h3>,
    h3: ({ children }) => <h4 data-md="3">{children}</h4>,
    h4: ({ children }) => <h5 data-md="4">{children}</h5>,
    a: ({ children, href }) => (
      <span className={styles.link} title={href}>
        {children}
      </span>
    ),
    img: ({ src, alt }) => {
      const target =
        source && typeof src === 'string' ? resolveImage(source.area, source.path, src) : null;
      return target && source ? (
        <NoteImage workspaceId={source.workspaceId} {...target} alt={alt ?? ''} />
      ) : (
        <span className={styles.missing}>{alt ?? 'Image'}</span>
      );
    },
    // A diagram isn't code, so it doesn't get the code block's frame.
    pre: ({ node, children }) => {
      const first = node?.children[0];
      const classes = first?.type === 'element' ? first.properties.className : undefined;
      const isDiagram = Array.isArray(classes) && classes.includes('language-mermaid');
      return isDiagram ? <>{children}</> : <pre className={styles.codeBlock}>{children}</pre>;
    },
    code: ({ className, children }) => {
      if (className === 'language-mermaid' && typeof children === 'string') {
        return <Mermaid code={children.replace(/\n$/, '')} />;
      }
      return <code className={className}>{children}</code>;
    },
  };
  return (
    <div className={styles.prose}>
      <ReactMarkdown remarkPlugins={[remarkGfm]} components={components}>
        {text}
      </ReactMarkdown>
    </div>
  );
}

function NoteImage({
  workspaceId,
  area,
  path,
  alt,
}: {
  workspaceId: string;
  area: Area;
  path: string;
  alt: string;
}) {
  const image = useKnowledgeImage(workspaceId, area, path);
  if (image.data) return <img src={image.data} alt={alt} className={styles.image} />;
  return <span className={styles.missing}>{image.isPending ? 'Loading image…' : alt || path}</span>;
}

/** Mermaid is large, so it loads only when a note has a diagram. */
function Mermaid({ code }: { code: string }) {
  const id = `mermaid-${useId().replace(/[^a-zA-Z0-9]/g, '')}`;
  const [svg, setSvg] = useState<string | null>(null);
  const [failed, setFailed] = useState(false);

  useEffect(() => {
    // An object, not a `let`: the effect's cleanup flips it while the render is awaited.
    const run = { cancelled: false };
    void (async () => {
      try {
        const { default: mermaid } = await import('mermaid');
        const dark = window.matchMedia('(prefers-color-scheme: dark)').matches;
        mermaid.initialize({
          startOnLoad: false,
          securityLevel: 'strict',
          theme: dark ? 'dark' : 'neutral',
          fontFamily: font.sans,
        });
        const out = await mermaid.render(id, code);
        if (!run.cancelled) {
          setSvg(out.svg);
          setFailed(false);
        }
      } catch {
        if (!run.cancelled) setFailed(true);
      }
    })();
    return () => {
      run.cancelled = true;
    };
  }, [code, id]);

  if (failed) {
    return (
      <pre className={styles.diagramError}>
        <span>This diagram has a mistake, so here’s its source:</span>
        {'\n'}
        {code}
      </pre>
    );
  }
  if (svg === null) return <div className={styles.diagram}>Drawing diagram…</div>;
  // Mermaid's strict mode sanitises its own SVG output.
  return <div className={styles.diagram} dangerouslySetInnerHTML={{ __html: svg }} />;
}
