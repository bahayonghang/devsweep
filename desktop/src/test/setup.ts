import "@testing-library/jest-dom/vitest";
import { cleanup } from "@testing-library/react";
import { afterEach } from "vitest";

afterEach(cleanup);

if (!HTMLDialogElement.prototype.showModal) {
  HTMLDialogElement.prototype.showModal = function showModal() { this.setAttribute("open", ""); };
}
if (!HTMLDialogElement.prototype.close) {
  HTMLDialogElement.prototype.close = function close() { this.removeAttribute("open"); };
}

// jsdom has no 2D canvas. Planet treats a null context as "draw nothing";
// Planet tests install their own context mock.
HTMLCanvasElement.prototype.getContext = function getContext() { return null; } as typeof HTMLCanvasElement.prototype.getContext;
