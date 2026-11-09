/**
 * Minimal typings for the parts of the self-hosted Ruffle bundle we use.
 * The full API types ship only for the npm-package import path; driving the
 * global <script> build through the custom element keeps the two worlds apart.
 */
interface RufflePlayerRegistry {
    newest?: () => RufflePlayerFactory;
}

interface RufflePlayerFactory {
    createPlayer: () => RufflePlayerElement;
}

interface RufflePlayerElement extends HTMLElement {
    load(options: Record<string, unknown>): Promise<unknown>;
}

declare global {
    interface Window {
        RufflePlayer?: RufflePlayerRegistry;
    }
}

export type {
    RufflePlayerRegistry,
    RufflePlayerFactory,
    RufflePlayerElement,
};
