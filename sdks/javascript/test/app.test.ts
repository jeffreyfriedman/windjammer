/**
 * Tests for app module.
 */

import { App } from '../src/app';

describe('App', () => {
  test('creation', () => {
    const app = new App();
    expect(app).toBeDefined();
    expect(app.isRunning()).toBe(false);
  });

  test('add system', () => {
    const app = new App();
    
    const called: boolean[] = [];
    app.addSystem(() => {
      called.push(true);
    });
    
    expect((app as any).systems.length).toBe(1);
  });

  test('add startup system', () => {
    const app = new App();
    
    app.addStartupSystem(() => {
      // Startup logic
    });
    
    expect((app as any).startupSystems.length).toBe(1);
  });

  test('add shutdown system', () => {
    const app = new App();
    
    app.addShutdownSystem(() => {
      // Shutdown logic
    });
    
    expect((app as any).shutdownSystems.length).toBe(1);
  });

  test('run', () => {
    const app = new App();
    
    const called: string[] = [];
    
    app.addStartupSystem(() => {
      called.push('startup');
    });
    
    app.addSystem(() => {
      called.push('update');
    });
    
    app.addShutdownSystem(() => {
      called.push('shutdown');
    });
    
    app.run();
    
    expect(called).toContain('startup');
    expect(called).toContain('update');
    expect(called).toContain('shutdown');
  });
});

