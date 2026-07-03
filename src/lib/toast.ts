import { createKumoToastManager } from "@cloudflare/kumo";

// 模块级 toast manager：在 <Toasty toastManager={...}> 注入，事件回调里可直接 dispatch，
// 不受 React 渲染周期 / 闭包陈旧影响。
export const toastManager = createKumoToastManager();
