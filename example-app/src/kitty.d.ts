declare module "*.kitty" {
  /** wasm-bindgen's generated `init` function; instantiates and runs the compiled Kittine app. */
  const init: (moduleOrPath?: unknown) => Promise<unknown>;
  export default init;
  export const bindings: Record<string, unknown>;
  export const crateName: string;
}
