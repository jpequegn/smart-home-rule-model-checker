import init, { analyze } from "../pkg/home_rule_wasm";
self.onmessage = async (e: MessageEvent) => {
  try {
    await init();
    const { operation, yaml, scenario } = e.data;
    self.postMessage({
      result: JSON.parse(analyze(operation, yaml, scenario)),
    });
  } catch (error) {
    self.postMessage({ error: String(error) });
  }
};
