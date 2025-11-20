#!/usr/bin/env ruby
# frozen_string_literal: true

# Hello World Example
#
# The simplest possible Windjammer application in Ruby.
#
# Run with: ruby examples/hello_world.rb

require 'windjammer'

puts '=== Windjammer Hello World (Ruby) ==='
puts "SDK Version: #{Windjammer::VERSION}"
puts

# Create a new application
app = Windjammer::App.new

# Add a simple system
app.add_system do
  puts 'Hello from the game loop!'
end

puts 'Application created successfully!'
puts 'Systems registered: 1'
puts
puts 'Note: Full app.run would start the game loop'
puts "For this example, we're just demonstrating SDK setup"
puts

# Run the application
app.run

