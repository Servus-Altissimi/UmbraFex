pub const DRAG_V: &str = "(function(){
    const p = document.querySelector('.zone-bottom');
    let y0 = event.clientY, h0 = p.getBoundingClientRect().height;
    const mm = e => p.style.height = Math.max(24, h0 - (e.clientY - y0)) + 'px';
    const mu = () => {
        removeEventListener('mousemove', mm);
        removeEventListener('mouseup', mu);
    };
    addEventListener('mousemove', mm);
    addEventListener('mouseup', mu);
})();";

pub const DRAG_H: &str = "(function(){
    const r = document.querySelector('.panel-right');
    let x0 = event.clientX, w0 = r.getBoundingClientRect().width;
    const mm = e => {
        r.style.width = Math.max(200, w0 - (e.clientX - x0)) + 'px';
        r.style.flex = 'none';
    };
    const mu = () => {
        removeEventListener('mousemove', mm);
        removeEventListener('mouseup', mu);
    };
    addEventListener('mousemove', mm);
    addEventListener('mouseup', mu);
})();";

pub const SYNC_SCROLL: &str = "
    const ta = document.querySelector('.code');
    const g  = document.querySelector('.gutter');
    const ov = document.querySelector('.highlight-overlay');
    g.scrollTop = ta.scrollTop;
    if (ov) { ov.scrollTop = ta.scrollTop; ov.scrollLeft = ta.scrollLeft; }
";

pub const CANVAS_SYNC: &str = "
    if (!window.__canvasSyncRunning) {
        window.__canvasSyncRunning = true;
        (function sync() {
            const slot  = document.getElementById('canvas-slot');
            const fixed = document.getElementById('canvas-fixed');
            if (slot && fixed) {
                const r = slot.getBoundingClientRect();
                if (r.width > 0 && r.height > 0) {
                    fixed.style.cssText =
                        'position:fixed;overflow:hidden;background:#000;z-index:1;' +
                        'left:'   + r.left   + 'px;' +
                        'top:'    + r.top    + 'px;' +
                        'width:'  + r.width  + 'px;' +
                        'height:' + r.height + 'px;';
                } else {
                    fixed.style.display = 'none';
                }
            } else if (fixed) {
                fixed.style.display = 'none'; // slot gone (no-webgpu pane showing)
            }
            requestAnimationFrame(sync);
        })();
    }
";

pub const FS_TOGGLE: &str = "
    const w = document.getElementById('canvas-fixed');
    if (!document.fullscreenElement) w.requestFullscreen();
    else document.exitFullscreen();
";

pub const LOAD_DOCK_STATE: &str = "
    try {
        const saved = localStorage.getItem('shader-dock-state');
        saved || '';
    } catch (e) {
        '';
    }
";

pub const ENABLE_BEFOREUNLOAD: &str = "
    window.__beforeUnloadHandler = (e) => {
        e.preventDefault();
        e.returnValue = '';
        return '';
    };
    window.addEventListener('beforeunload', window.__beforeUnloadHandler);
";

pub const DISABLE_BEFOREUNLOAD: &str = "
    if (window.__beforeUnloadHandler) {
        window.removeEventListener('beforeunload', window.__beforeUnloadHandler);
        delete window.__beforeUnloadHandler;
    }
";
