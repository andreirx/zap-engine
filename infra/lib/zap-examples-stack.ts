import * as cdk from 'aws-cdk-lib';
import * as s3 from 'aws-cdk-lib/aws-s3';
import * as s3deploy from 'aws-cdk-lib/aws-s3-deployment';
import * as cloudfront from 'aws-cdk-lib/aws-cloudfront';
import * as origins from 'aws-cdk-lib/aws-cloudfront-origins';
import { Construct } from 'constructs';

export class ZapExamplesStack extends cdk.Stack {
  constructor(scope: Construct, id: string, props?: cdk.StackProps) {
    super(scope, id, props);

    // S3 bucket — private, CloudFront-only access
    const siteBucket = new s3.Bucket(this, 'ZapExamplesBucket', {
      publicReadAccess: false,
      blockPublicAccess: s3.BlockPublicAccess.BLOCK_ALL,
      removalPolicy: cdk.RemovalPolicy.DESTROY,
      autoDeleteObjects: true,
    });

    // Response headers: COOP + COEP required for SharedArrayBuffer
    const engineHeaders = new cloudfront.ResponseHeadersPolicy(this, 'EngineHeaders', {
      securityHeadersBehavior: {
        contentTypeOptions: { override: true },
        frameOptions: {
          frameOption: cloudfront.HeadersFrameOption.DENY,
          override: true,
        },
        strictTransportSecurity: {
          accessControlMaxAge: cdk.Duration.days(365),
          includeSubdomains: true,
          override: true,
        },
      },
      customHeadersBehavior: {
        customHeaders: [
          { header: 'Cross-Origin-Opener-Policy', value: 'same-origin', override: true },
          { header: 'Cross-Origin-Embedder-Policy', value: 'require-corp', override: true },
        ],
      },
    });

    // CloudFront distribution
    const distribution = new cloudfront.Distribution(this, 'ZapExamplesDist', {
      defaultBehavior: {
        origin: origins.S3BucketOrigin.withOriginAccessControl(siteBucket),
        viewerProtocolPolicy: cloudfront.ViewerProtocolPolicy.REDIRECT_TO_HTTPS,
        responseHeadersPolicy: engineHeaders,
        compress: true,
      },
      defaultRootObject: 'index.html',
      // SPA fallback for nested example routes
      errorResponses: [
        {
          httpStatus: 403,
          responseHttpStatus: 200,
          responsePagePath: '/index.html',
          ttl: cdk.Duration.seconds(0),
        },
      ],
    });

    // Deploy dist/ folder to S3
    new s3deploy.BucketDeployment(this, 'DeployExamples', {
      sources: [s3deploy.Source.asset('../dist')],
      destinationBucket: siteBucket,
      distribution,
      distributionPaths: ['/*'],
    });

    // Output the URL
    new cdk.CfnOutput(this, 'SiteURL', {
      value: `https://${distribution.distributionDomainName}`,
    });
  }
}
