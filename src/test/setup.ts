import { cleanup } from "@testing-library/react";
import { afterEach } from "vitest";

// Globals are off, so RTL's automatic cleanup is not registered; do it explicitly.
afterEach(() => cleanup());
