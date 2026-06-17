export interface PrintableFrame {
  style: {
    position?: string;
    width?: string;
    height?: string;
    border?: string;
  };
  contentDocument: {
    open: () => void;
    write: (markup: string) => void;
    close: () => void;
  } | null;
  remove: () => void;
}

export interface PrintFrameHost {
  document: {
    createElement: (tagName: 'iframe') => PrintableFrame;
    body: {
      appendChild: (node: PrintableFrame) => void;
    };
  };
}

function defaultPrintFrameHost(): PrintFrameHost {
  return {
    document: document as unknown as PrintFrameHost['document'],
  };
}

export function printableCaptureDocument(imageBase64: string) {
  return `<!doctype html>
<html>
  <head>
    <meta charset="utf-8" />
    <title>SnapLingo Print</title>
    <style>
      html,
      body {
        margin: 0;
        min-height: 100%;
      }

      body {
        display: grid;
        place-items: center;
      }

      img {
        max-width: 100vw;
        max-height: 100vh;
      }
    </style>
  </head>
  <body>
    <img
      src="data:image/png;base64,${imageBase64}"
      alt=""
      onload="window.focus(); window.print(); setTimeout(() => window.frameElement.remove(), 0)"
    />
  </body>
</html>`;
}

export function printBase64PngImage(
  imageBase64: string,
  host: PrintFrameHost = defaultPrintFrameHost(),
) {
  const frame = host.document.createElement('iframe');
  frame.style.position = 'fixed';
  frame.style.width = '0';
  frame.style.height = '0';
  frame.style.border = '0';
  host.document.body.appendChild(frame);

  if (!frame.contentDocument) {
    frame.remove();
    throw new Error('Failed to prepare print frame');
  }

  frame.contentDocument.open();
  frame.contentDocument.write(printableCaptureDocument(imageBase64));
  frame.contentDocument.close();
}
