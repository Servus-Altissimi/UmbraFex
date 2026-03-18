pub const DRAG_V: &str = "(function(){
    const p = document.querySelector('.pane-errors');
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
