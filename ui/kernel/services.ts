// The host services, mounted before any plugin: what plugins build on.
import type { Plugin } from "@neoworks/extension-system";
import { AppService } from "./services/app.ts";
import { KeyboardService } from "./services/keyboard.ts";
import { KeymapService } from "./services/keymap.ts";
import { SlotsService } from "./slots/service.ts";

export const services: Plugin[] = [SlotsService, AppService, KeymapService, KeyboardService];
