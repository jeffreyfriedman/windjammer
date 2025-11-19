#!/usr/bin/env python3
"""
Windjammer Python SDK Setup
"""

from setuptools import setup, find_packages
import os

# Read the README file
def read_readme():
    readme_path = os.path.join(os.path.dirname(__file__), 'README.md')
    if os.path.exists(readme_path):
        with open(readme_path, 'r', encoding='utf-8') as f:
            return f.read()
    return ''

setup(
    name='windjammer-sdk',
    version='0.1.0',
    description='Python SDK for Windjammer Game Engine',
    long_description=read_readme(),
    long_description_content_type='text/markdown',
    author='Windjammer Contributors',
    author_email='contact@windjammer.dev',
    url='https://github.com/windjammer/windjammer',
    project_urls={
        'Documentation': 'https://windjammer.dev/docs/python',
        'Source': 'https://github.com/windjammer/windjammer',
        'Tracker': 'https://github.com/windjammer/windjammer/issues',
    },
    packages=find_packages(),
    python_requires='>=3.8',
    install_requires=[
        'cffi>=1.15.0',
        'numpy>=1.20.0',
    ],
    extras_require={
        'dev': [
            'pytest>=7.0.0',
            'pytest-cov>=4.0.0',
            'black>=22.0.0',
            'mypy>=1.0.0',
            'pylint>=2.15.0',
        ],
    },
    classifiers=[
        'Development Status :: 3 - Alpha',
        'Intended Audience :: Developers',
        'Topic :: Games/Entertainment',
        'Topic :: Software Development :: Libraries :: Python Modules',
        'License :: OSI Approved :: MIT License',
        'License :: OSI Approved :: Apache Software License',
        'Programming Language :: Python :: 3',
        'Programming Language :: Python :: 3.8',
        'Programming Language :: Python :: 3.9',
        'Programming Language :: Python :: 3.10',
        'Programming Language :: Python :: 3.11',
        'Programming Language :: Python :: 3.12',
        'Operating System :: OS Independent',
    ],
    keywords='game engine gamedev 2d 3d graphics windjammer',
    include_package_data=True,
    zip_safe=False,
)

