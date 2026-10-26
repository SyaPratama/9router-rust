/*
 * Dashboard application layer.
 * Page modules are represented by stable route metadata so the embedded UI
 * follows the upstream layout -> page -> API-client flow without a framework
 * runtime dependency.
 */
(function () {
  const pages = [
    { id: "overview", path: "endpoint", group: "Gateway" },
    { id: "providers", path: "providers", group: "Gateway" },
    { id: "combos", path: "combos", group: "Gateway" },
    { id: "usage", path: "usage", group: "Gateway" },
    { id: "quota", path: "quota", group: "Gateway" },
    { id: "token-saver", path: "token-saver", group: "Gateway" },
    { id: "cli-tools", path: "cli-tools", group: "Tools" },
    { id: "translator", path: "translator", group: "Tools" },
    { id: "console-log", path: "console-log", group: "Tools" },
    { id: "media", path: "media-providers/web", group: "System" },
    { id: "proxy-pools", path: "proxy-pools", group: "System" },
    { id: "settings", path: "profile", group: "System" }
  ];

  window.NineRouterDashboard = Object.freeze({
    pages,
    mount(rootId) {
      const root = document.getElementById(rootId);
      if (root) root.innerHTML = '<div class="card" style="margin:32px">Dashboard shell loading… <a href="/dashboard">Continue</a></div>';
    },
    findPage(id) { return pages.find(page => page.id === id); },
    api: {
      get(path) { return fetch(path).then(response => response.json()); },
      json(path, options) {
        return fetch(path, {
          headers: { "Content-Type": "application/json" },
          ...options
        }).then(async response => {
          if (!response.ok) throw new Error(await response.text());
          return response.json();
        });
      }
    }
  });

  document.documentElement.dataset.dashboardArchitecture = "layout-page-api";
})();
