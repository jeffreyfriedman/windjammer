/**
 * Hello World Example
 * 
 * The simplest possible Windjammer application in C#.
 * 
 * Run with: dotnet run --project Examples/HelloWorld.cs
 */

using System;
using Windjammer.SDK;

namespace Windjammer.Examples
{
    class HelloWorld
    {
        static void Main(string[] args)
        {
            Console.WriteLine("=== Windjammer Hello World (C#) ===");
            Console.WriteLine($"SDK Version: {App.VERSION}");
            Console.WriteLine();

            // Create a new application
            var app = new App();

            // Add a simple system
            app.AddSystem(() =>
            {
                Console.WriteLine("Hello from the game loop!");
            });

            Console.WriteLine("Application created successfully!");
            Console.WriteLine("Systems registered: 1");
            Console.WriteLine();
            Console.WriteLine("Note: Full app.Run() would start the game loop");
            Console.WriteLine("For this example, we're just demonstrating SDK setup");
            Console.WriteLine();

            // Run the application
            app.Run();
        }
    }
}

