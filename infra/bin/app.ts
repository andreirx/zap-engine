#!/usr/bin/env node
import * as cdk from 'aws-cdk-lib';
import { ZapExamplesStack } from '../lib/zap-examples-stack';

const app = new cdk.App();
new ZapExamplesStack(app, 'ZapExamplesStack');
