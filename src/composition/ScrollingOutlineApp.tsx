import { ScrollingCaptureOutlineWindow } from '../views/ScrollingCaptureWindow';

const parameters = new URLSearchParams(window.location.search);
export default function ScrollingOutlineApp() {
  return (
    <ScrollingCaptureOutlineWindow
      scale={Number(parameters.get('scale') ?? 1)}
      desktopScale={Number(parameters.get('desktopScale') ?? 0)}
    />
  );
}
