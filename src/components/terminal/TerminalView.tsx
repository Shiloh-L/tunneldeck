import { useEffect, useRef } from 'react';
import { Terminal } from '@xterm/xterm';
import { FitAddon } from '@xterm/addon-fit';
import { WebLinksAddon } from '@xterm/addon-web-links';
import { listen } from '@tauri-apps/api/event';
import type { TerminalDataEvent, TerminalExitEvent } from '@/types';
import * as api from '@/lib/tauri';
import '@xterm/xterm/css/xterm.css';

interface TerminalViewProps {
  terminalId: string;
  isActive: boolean;
}

export function TerminalView({ terminalId, isActive }: TerminalViewProps) {
  const containerRef = useRef<HTMLDivElement>(null);
  const termRef = useRef<Terminal | null>(null);
  const fitAddonRef = useRef<FitAddon | null>(null);

  // Initialize xterm.js
  useEffect(() => {
    if (!containerRef.current) return;

    const term = new Terminal({
      fontFamily: 'Consolas, "Courier New", monospace',
      fontSize: 13,
      lineHeight: 1.2,
      theme: {
        background: '#0d1117',
        foreground: '#c9d1d9',
        cursor: '#58a6ff',
        selectionBackground: '#264f78',
        black: '#484f58',
        red: '#ff7b72',
        green: '#3fb950',
        yellow: '#d29922',
        blue: '#58a6ff',
        magenta: '#bc8cff',
        cyan: '#39c5cf',
        white: '#b1bac4',
        brightBlack: '#6e7681',
        brightRed: '#ffa198',
        brightGreen: '#56d364',
        brightYellow: '#e3b341',
        brightBlue: '#79c0ff',
        brightMagenta: '#d2a8ff',
        brightCyan: '#56d4dd',
        brightWhite: '#f0f6fc',
      },
      cursorBlink: true,
      scrollback: 10000,
    });

    const fitAddon = new FitAddon();
    const webLinksAddon = new WebLinksAddon();

    term.loadAddon(fitAddon);
    term.loadAddon(webLinksAddon);
    term.open(containerRef.current);

    requestAnimationFrame(() => {
      fitAddon.fit();
    });

    // Send user input to Rust backend
    term.onData((data) => {
      api.writeTerminal(terminalId, data);
    });

    // Send resize events to Rust backend
    term.onResize(({ cols, rows }) => {
      api.resizeTerminal(terminalId, cols, rows);
    });

    termRef.current = term;
    fitAddonRef.current = fitAddon;

    return () => {
      term.dispose();
    };
  }, [terminalId]);

  // Listen for terminal data from Rust
  useEffect(() => {
    const unlistenData = listen<TerminalDataEvent>('terminal-data', (event) => {
      if (event.payload.terminalId === terminalId && termRef.current) {
        // Decode base64 to raw bytes — preserves non-UTF-8 terminal data
        const binaryStr = atob(event.payload.data);
        const bytes = new Uint8Array(binaryStr.length);
        for (let i = 0; i < binaryStr.length; i++) {
          bytes[i] = binaryStr.charCodeAt(i);
        }
        termRef.current.write(bytes);
      }
    });

    const unlistenExit = listen<TerminalExitEvent>('terminal-exit', (event) => {
      if (event.payload.terminalId === terminalId && termRef.current) {
        termRef.current.write('\r\n\x1b[31m[会话已关闭]\x1b[0m\r\n');
      }
    });

    return () => {
      unlistenData.then((f) => f());
      unlistenExit.then((f) => f());
    };
  }, [terminalId]);

  // Refit when becoming active
  useEffect(() => {
    if (isActive && fitAddonRef.current) {
      requestAnimationFrame(() => {
        fitAddonRef.current?.fit();
      });
    }
  }, [isActive]);

  // Window resize handler
  useEffect(() => {
    const handleResize = () => {
      if (isActive && fitAddonRef.current) {
        fitAddonRef.current.fit();
      }
    };
    window.addEventListener('resize', handleResize);
    return () => window.removeEventListener('resize', handleResize);
  }, [isActive]);

  return (
    <div
      ref={containerRef}
      className='h-full w-full'
      style={{ display: isActive ? 'block' : 'none' }}
    />
  );
}
